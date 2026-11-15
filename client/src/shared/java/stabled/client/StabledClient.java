package stabled.client;

import com.google.gson.JsonObject;
import com.mojang.blaze3d.platform.InputConstants;
import java.nio.file.Files;
import java.util.function.BooleanSupplier;
import java.util.function.Consumer;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.TitleScreen;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import stabled.client.config.ClientSettings;
import stabled.client.config.JsonFiles;
import stabled.client.config.Onboarding;
import stabled.client.config.RuntimeStatus;
import stabled.client.config.StabledPaths;
import stabled.client.discord.PresenceManager;
import stabled.client.emote.EmotePlayer;
import stabled.client.hud.FpsHud;
import stabled.client.platform.MotionBlur;
import stabled.client.platform.Platform;
import stabled.client.theme.ThemeManager;
import stabled.client.ui.screen.EmoteWheelScreen;
import stabled.client.ui.screen.IntroScreen;
import stabled.client.ui.screen.StabledSettingsScreen;
import stabled.client.ui.screen.TitleLayout;
import stabled.client.ui.StabledStyle;

/** Stabled Client entry point. */
public final class StabledClient implements ClientModInitializer {
	public static final String MOD_ID = "stabled";
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");

	private static ClientSettings settings;
	private static Onboarding onboarding;
	private static String saveName = "Stabled";
	private static boolean ready;
	private static boolean introShownThisSession;
	private static boolean introOpen;
	private static boolean runtimeDirty = true;
	private static BooleanSupplier openSettingsKey;
	private static KeyMapping emoteKey;
	private static int tick;

	@Override
	public void onInitializeClient() {
		StabledPaths.init();
		ThemeManager.init();
		boolean firstRun = !Files.isRegularFile(StabledPaths.clientSettingsFile());
		settings = ClientSettings.load();
		if (firstRun) {
			settings.save(); // Documented defaults for a Save created outside the launcher (development runs).
		}
		onboarding = Onboarding.load();
		JsonObject save = JsonFiles.readObject(StabledPaths.saveDir().resolve("save.json"));
		if (save.has("name") && save.get("name").isJsonPrimitive()) {
			saveName = save.get("name").getAsString();
		}
		ready = true;

		Platform.registerHud("stabled:fps", FpsHud::extract);
		Platform.registerHud("stabled:emote", EmotePlayer::drawHud);
		openSettingsKey = Platform.registerKey("key.stabled.settings");
		emoteKey = Platform.registerMapping("key.stabled.emotes", InputConstants.KEY_B);

		PresenceManager.get().start(settings.discordEnabled, settings.discordShowElapsed);
		ClientLifecycleEvents.CLIENT_STARTED.register(client -> applyPresetIfNeeded());
		ClientTickEvents.END_CLIENT_TICK.register(StabledClient::onTick);
		// Mods like Mod Menu and In-Game Account Switcher add title-screen buttons after Stabled's layout ran.
		Platform.afterScreenInit(screen -> {
			if (screen instanceof TitleScreen title && StabledStyle.menusStyled()) {
				TitleLayout.relayout(title);
			}
		});
		ClientLifecycleEvents.CLIENT_STOPPING.register(client -> {
			PresenceManager.get().shutdown();
			RuntimeStatus.write("stopped", null, MotionBlur.unavailableReason(), false);
		});
		LOGGER.info("Stabled Client ready (launched by Stabled: {})", StabledPaths.launchedByStabled());
	}

	public static boolean isReady() {
		return ready;
	}

	/** The emote wheel key (B by default; players can rebind it in Controls). */
	public static KeyMapping emoteKey() {
		return emoteKey;
	}

	public static ClientSettings settings() {
		return settings;
	}

	public static Onboarding onboarding() {
		return onboarding;
	}

	public static String saveName() {
		return saveName;
	}

	public static void setIntroOpen(boolean open) {
		introOpen = open;
		runtimeDirty = true;
	}

	/** Applies a change to this Save's settings, persists it immediately and updates live integrations. */
	public static void update(Consumer<ClientSettings> change) {
		change.accept(settings);
		settings.save();
		PresenceManager.get().setEnabled(settings.discordEnabled);
		PresenceManager.get().setShowElapsed(settings.discordShowElapsed);
		runtimeDirty = true;
	}

	private static void applyPresetIfNeeded() {
		// Presets are applied once when chosen (e.g. in the launcher before first launch), never over manual tweaks.
		if (!"custom".equals(settings.performancePreset) && !settings.performancePreset.equals(settings.performanceApplied)) {
			Platform.applyPreset(settings.performancePreset);
			settings.performanceApplied = settings.performancePreset;
			settings.save();
		}
	}

	private static void onTick(Minecraft mc) {
		while (openSettingsKey.getAsBoolean()) {
			if (Platform.screen() == null) {
				Platform.setScreen(new StabledSettingsScreen(null));
			}
		}
		while (emoteKey.consumeClick()) {
			if (Platform.screen() == null && mc.player != null) {
				Platform.setScreen(new EmoteWheelScreen(emoteKey));
			}
		}
		EmotePlayer.tick(mc);
		if (!introShownThisSession && !onboarding.completed && Platform.screen() instanceof TitleScreen title) {
			introShownThisSession = true;
			Platform.setScreen(new IntroScreen(title));
		}
		if (++tick % 10 == 0) {
			PresenceManager.get().setActivity(activityFor(mc));
		}
		if (PresenceManager.get().consumeStatusChange()) {
			runtimeDirty = true;
		}
		if (runtimeDirty && tick % 20 == 0) {
			runtimeDirty = false;
			RuntimeStatus.write(PresenceManager.get().status().name().toLowerCase(java.util.Locale.ROOT), PresenceManager.get().detail(),
				MotionBlur.unavailableReason(), introOpen);
		}
	}

	private static PresenceManager.Activity activityFor(Minecraft mc) {
		if (mc.level == null) {
			return PresenceManager.Activity.MENUS;
		}
		if (mc.hasSingleplayerServer()) {
			return PresenceManager.Activity.SINGLEPLAYER;
		}
		return Platform.playingOnRealms() ? PresenceManager.Activity.REALMS : PresenceManager.Activity.MULTIPLAYER;
	}
}
