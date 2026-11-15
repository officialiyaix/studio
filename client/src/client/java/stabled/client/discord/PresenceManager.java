package stabled.client.discord;

import com.google.gson.JsonObject;
import java.io.IOException;
import java.time.Instant;
import java.util.Objects;
import java.util.UUID;
import java.util.regex.Pattern;
import net.fabricmc.loader.api.FabricLoader;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Owns the single Discord presence connection for this game process.
 *
 * <ul>
 *   <li>One worker thread connects, publishes and clears activity; one reader thread per connection answers pings.</li>
 *   <li>Retries use exponential backoff (5 s up to 60 s), so a closed Discord never causes busy polling.</li>
 *   <li>Activity updates are coalesced to at most one per 2 s, inside Discord's documented rate limit.</li>
 *   <li>Turning presence off clears the activity immediately and closes the connection.</li>
 *   <li>Failures never reach the game thread; Minecraft keeps running normally.</li>
 * </ul>
 */
public final class PresenceManager {
	public enum Status { DISABLED, NOT_CONFIGURED, CONNECTING, CONNECTED, WAITING_FOR_DISCORD, REJECTED }

	public enum Activity {
		MENUS("In menus"),
		SINGLEPLAYER("Playing singleplayer"),
		MULTIPLAYER("Playing multiplayer"),
		REALMS("Playing on Realms");

		public final String details;

		Activity(String details) {
			this.details = details;
		}
	}

	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");
	private static final Pattern SNOWFLAKE = Pattern.compile("\\d{17,20}");
	private static final long MIN_BACKOFF_MS = 5_000L;
	private static final long MAX_BACKOFF_MS = 60_000L;
	private static final long UPDATE_INTERVAL_MS = 2_000L;
	private static final PresenceManager INSTANCE = new PresenceManager();

	private final Object monitor = new Object();
	private final @Nullable String applicationId;
	private final long sessionStartSeconds = Instant.now().getEpochSecond();
	private final String gameVersion;
	private volatile boolean enabled;
	private volatile boolean showElapsed = true;
	private volatile Activity activity = Activity.MENUS;
	private volatile boolean running;
	private volatile Status status = Status.DISABLED;
	private volatile long nextRetryAt;
	private volatile @Nullable String detail;
	private volatile boolean statusChanged = true;
	private @Nullable DiscordIpc ipc;
	private @Nullable Thread worker;

	private PresenceManager() {
		String id = System.getProperty("stabled.discordAppId", "").trim();
		this.applicationId = SNOWFLAKE.matcher(id).matches() ? id : null;
		this.gameVersion = FabricLoader.getInstance().getModContainer("minecraft")
			.map(c -> c.getMetadata().getVersion().getFriendlyString()).orElse("");
	}

	public static PresenceManager get() {
		return INSTANCE;
	}

	public boolean isConfigured() {
		return this.applicationId != null;
	}

	public void start(boolean enabled, boolean showElapsed) {
		this.enabled = enabled;
		this.showElapsed = showElapsed;
		if (this.running) {
			return;
		}
		this.running = true;
		Thread t = new Thread(this::run, "Stabled Discord Presence");
		t.setDaemon(true);
		this.worker = t;
		t.start();
	}

	public void setEnabled(boolean enabled) {
		this.enabled = enabled;
		this.wake();
	}

	public void setShowElapsed(boolean showElapsed) {
		this.showElapsed = showElapsed;
		this.wake();
	}

	public void setActivity(Activity activity) {
		if (this.activity != activity) {
			this.activity = activity;
			this.wake();
		}
	}

	public Activity activity() {
		return this.activity;
	}

	public long sessionStartSeconds() {
		return this.sessionStartSeconds;
	}

	public String gameVersion() {
		return this.gameVersion;
	}

	public Status status() {
		return this.status;
	}

	public @Nullable String detail() {
		return this.detail;
	}

	public long secondsUntilRetry() {
		return Math.max(0, (this.nextRetryAt - System.currentTimeMillis() + 999) / 1000);
	}

	/** True once since the last call; lets the game thread mirror status into runtime.json without polling. */
	public boolean consumeStatusChange() {
		boolean changed = this.statusChanged;
		this.statusChanged = false;
		return changed;
	}

	/** Clears activity and closes the connection. Called when the game stops. */
	public void shutdown() {
		this.running = false;
		this.wake();
		Thread t = this.worker;
		if (t != null) {
			try {
				t.join(1500);
			} catch (InterruptedException e) {
				Thread.currentThread().interrupt();
			}
		}
		this.disconnect(true);
	}

	private void wake() {
		synchronized (this.monitor) {
			this.monitor.notifyAll();
		}
	}

	private void setStatus(Status status, @Nullable String detail) {
		if (this.status != status || !Objects.equals(this.detail, detail)) {
			this.status = status;
			this.detail = detail;
			this.statusChanged = true;
		}
	}

	private void run() {
		long backoff = MIN_BACKOFF_MS;
		String lastSent = null;
		long lastSendAt = 0L;
		while (this.running) {
			try {
				if (this.applicationId == null || !this.enabled) {
					this.disconnect(true);
					lastSent = null;
					this.setStatus(this.applicationId == null ? Status.NOT_CONFIGURED : Status.DISABLED, null);
					this.await(0L);
					continue;
				}
				if (this.ipc == null) {
					this.setStatus(Status.CONNECTING, null);
					try {
						this.connect();
						backoff = MIN_BACKOFF_MS;
						lastSent = null;
						this.setStatus(Status.CONNECTED, null);
					} catch (RejectedException e) {
						this.nextRetryAt = System.currentTimeMillis() + MAX_BACKOFF_MS;
						this.setStatus(Status.REJECTED, e.getMessage());
						this.await(MAX_BACKOFF_MS);
						continue;
					} catch (IOException e) {
						this.disconnect(false);
						this.nextRetryAt = System.currentTimeMillis() + backoff;
						this.setStatus(Status.WAITING_FOR_DISCORD, null);
						this.await(backoff);
						backoff = Math.min(MAX_BACKOFF_MS, backoff * 2);
						continue;
					}
				}
				String key = this.activity.name() + "|" + this.showElapsed;
				if (!key.equals(lastSent)) {
					long wait = lastSendAt + UPDATE_INTERVAL_MS - System.currentTimeMillis();
					if (wait > 0) {
						this.await(wait);
						continue;
					}
					this.sendActivity();
					lastSent = key;
					lastSendAt = System.currentTimeMillis();
				}
				this.await(0L);
			} catch (IOException e) {
				LOGGER.info("Stabled: Discord connection lost ({}); will retry", e.getMessage());
				this.disconnect(false);
				lastSent = null;
			} catch (InterruptedException e) {
				Thread.currentThread().interrupt();
				return;
			} catch (RuntimeException e) {
				LOGGER.warn("Stabled: unexpected Discord presence error", e);
				this.disconnect(false);
				lastSent = null;
				try {
					this.await(MAX_BACKOFF_MS);
				} catch (InterruptedException ie) {
					return;
				}
			}
		}
	}

	private void await(long millis) throws InterruptedException {
		synchronized (this.monitor) {
			if (!this.running) {
				return;
			}
			if (millis <= 0) {
				this.monitor.wait();
			} else {
				this.monitor.wait(millis);
			}
		}
	}

	private static final class RejectedException extends IOException {
		RejectedException(String message) {
			super(message);
		}
	}

	private void connect() throws IOException {
		DiscordIpc connection = DiscordIpc.open();
		try {
			JsonObject hello = new JsonObject();
			hello.addProperty("v", 1);
			hello.addProperty("client_id", this.applicationId);
			connection.send(DiscordIpc.OP_HANDSHAKE, hello);
			DiscordIpc.Frame ready = connection.read();
			if (ready.opcode() == DiscordIpc.OP_CLOSE) {
				String message = ready.body().has("message") ? ready.body().get("message").getAsString() : "Discord closed the connection";
				throw new RejectedException("Discord rejected the application ID: " + message);
			}
			if (!ready.body().has("evt") || !"READY".equals(ready.body().get("evt").getAsString())) {
				throw new IOException("Unexpected Discord handshake response");
			}
		} catch (IOException e) {
			connection.close();
			throw e;
		}
		this.ipc = connection;
		Thread reader = new Thread(() -> this.readLoop(connection), "Stabled Discord Reader");
		reader.setDaemon(true);
		reader.start();
	}

	private void readLoop(DiscordIpc connection) {
		try {
			while (this.running && this.ipc == connection) {
				DiscordIpc.Frame frame = connection.read();
				if (frame.opcode() == DiscordIpc.OP_PING) {
					connection.send(DiscordIpc.OP_PONG, frame.body());
				} else if (frame.opcode() == DiscordIpc.OP_CLOSE) {
					break;
				}
			}
		} catch (IOException | RuntimeException ignored) {
			// Connection closed; the worker reconnects with backoff.
		}
		if (this.ipc == connection) {
			this.ipc = null;
			connection.close();
			this.wake();
		}
	}

	private void sendActivity() throws IOException {
		DiscordIpc connection = this.ipc;
		if (connection == null) {
			return;
		}
		JsonObject activity = new JsonObject();
		activity.addProperty("type", 0);
		activity.addProperty("details", this.activity.details);
		if (!this.gameVersion.isEmpty()) {
			activity.addProperty("state", "Minecraft " + this.gameVersion);
		}
		if (this.showElapsed) {
			JsonObject timestamps = new JsonObject();
			timestamps.addProperty("start", this.sessionStartSeconds);
			activity.add("timestamps", timestamps);
		}
		JsonObject assets = new JsonObject();
		assets.addProperty("large_image", "stabled");
		assets.addProperty("large_text", "Stabled Client");
		activity.add("assets", assets);
		connection.send(DiscordIpc.OP_FRAME, command(activity));
	}

	private static JsonObject command(@Nullable JsonObject activity) {
		JsonObject args = new JsonObject();
		args.addProperty("pid", ProcessHandle.current().pid());
		if (activity != null) {
			args.add("activity", activity);
		}
		JsonObject cmd = new JsonObject();
		cmd.addProperty("cmd", "SET_ACTIVITY");
		cmd.add("args", args);
		cmd.addProperty("nonce", UUID.randomUUID().toString());
		return cmd;
	}

	private void disconnect(boolean clearFirst) {
		DiscordIpc connection = this.ipc;
		this.ipc = null;
		if (connection != null) {
			if (clearFirst) {
				try {
					connection.send(DiscordIpc.OP_FRAME, command(null));
				} catch (IOException ignored) {
				}
			}
			connection.close();
		}
	}
}
