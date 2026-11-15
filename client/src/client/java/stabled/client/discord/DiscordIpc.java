package stabled.client.discord;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.EOFException;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.net.StandardProtocolFamily;
import java.net.UnixDomainSocketAddress;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.channels.SocketChannel;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * Minimal client for the Discord desktop RPC (IPC) protocol.
 *
 * <p>Frames are {@code int32 opcode, int32 length} (little-endian) followed by UTF-8 JSON. Only the local Discord
 * client is contacted, using the public application ID; no Discord account token or password is ever involved.
 */
final class DiscordIpc implements AutoCloseable {
	static final int OP_HANDSHAKE = 0;
	static final int OP_FRAME = 1;
	static final int OP_CLOSE = 2;
	static final int OP_PING = 3;
	static final int OP_PONG = 4;

	private interface Transport extends AutoCloseable {
		void write(byte[] data) throws IOException;

		void readFully(byte[] data) throws IOException;

		@Override
		void close() throws IOException;
	}

	record Frame(int opcode, JsonObject body) {
	}

	private final Transport transport;
	private final Object writeLock = new Object();

	private DiscordIpc(Transport transport) {
		this.transport = transport;
	}

	/** Tries discord-ipc-0..9 on the current platform. Throws if Discord is not running. */
	static DiscordIpc open() throws IOException {
		IOException last = null;
		boolean windows = System.getProperty("os.name", "").toLowerCase(Locale.ROOT).contains("win");
		for (int i = 0; i < 10; i++) {
			try {
				if (windows) {
					return new DiscordIpc(new PipeTransport(new RandomAccessFile("\\\\.\\pipe\\discord-ipc-" + i, "rw")));
				}
				for (Path dir : unixCandidates()) {
					Path socket = dir.resolve("discord-ipc-" + i);
					if (Files.exists(socket)) {
						SocketChannel channel = SocketChannel.open(StandardProtocolFamily.UNIX);
						try {
							channel.connect(UnixDomainSocketAddress.of(socket));
							return new DiscordIpc(new UnixTransport(channel));
						} catch (IOException e) {
							channel.close();
							last = e;
						}
					}
				}
			} catch (IOException e) {
				last = e;
			}
		}
		throw last != null ? last : new IOException("Discord is not running");
	}

	private static List<Path> unixCandidates() {
		List<Path> dirs = new ArrayList<>();
		for (String env : new String[] {"XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"}) {
			String v = System.getenv(env);
			if (v != null && !v.isBlank()) {
				Path base = Path.of(v);
				dirs.add(base);
				dirs.add(base.resolve("app/com.discordapp.Discord"));
				dirs.add(base.resolve("snap.discord"));
				dirs.add(base.resolve(".flatpak/com.discordapp.Discord/xdg-run"));
			}
		}
		dirs.add(Path.of("/tmp"));
		return dirs;
	}

	void send(int opcode, JsonObject body) throws IOException {
		byte[] payload = body.toString().getBytes(StandardCharsets.UTF_8);
		ByteBuffer buf = ByteBuffer.allocate(8 + payload.length).order(ByteOrder.LITTLE_ENDIAN);
		buf.putInt(opcode).putInt(payload.length).put(payload);
		synchronized (this.writeLock) {
			this.transport.write(buf.array());
		}
	}

	Frame read() throws IOException {
		byte[] header = new byte[8];
		this.transport.readFully(header);
		ByteBuffer h = ByteBuffer.wrap(header).order(ByteOrder.LITTLE_ENDIAN);
		int opcode = h.getInt();
		int length = h.getInt();
		if (length < 0 || length > 1 << 20) {
			throw new IOException("Invalid Discord IPC frame length " + length);
		}
		byte[] payload = new byte[length];
		this.transport.readFully(payload);
		String json = new String(payload, StandardCharsets.UTF_8);
		JsonObject body = json.isBlank() ? new JsonObject() : JsonParser.parseString(json).getAsJsonObject();
		return new Frame(opcode, body);
	}

	@Override
	public void close() {
		try {
			this.transport.close();
		} catch (IOException ignored) {
		}
	}

	private record PipeTransport(RandomAccessFile file) implements Transport {
		@Override
		public void write(byte[] data) throws IOException {
			this.file.write(data);
		}

		@Override
		public void readFully(byte[] data) throws IOException {
			this.file.readFully(data);
		}

		@Override
		public void close() throws IOException {
			this.file.close();
		}
	}

	private record UnixTransport(SocketChannel channel) implements Transport {
		@Override
		public void write(byte[] data) throws IOException {
			ByteBuffer b = ByteBuffer.wrap(data);
			while (b.hasRemaining()) {
				this.channel.write(b);
			}
		}

		@Override
		public void readFully(byte[] data) throws IOException {
			ByteBuffer b = ByteBuffer.wrap(data);
			while (b.hasRemaining()) {
				if (this.channel.read(b) < 0) {
					throw new EOFException("Discord closed the connection");
				}
			}
		}

		@Override
		public void close() throws IOException {
			this.channel.close();
		}
	}
}
