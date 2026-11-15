package stabled.client;

import com.google.gson.JsonObject;
import com.mojang.blaze3d.platform.InputConstants;
import java.nio.file.Files;
import java.util.function.Consumer;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.resources.Identifier;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import stabled.client.config.ClientSettings;
import stabled.client.config.JsonFiles;
import stabled.client.config.Onboarding;
import stabled.client.config.RuntimeStatus;
import stabled.client.config.StabledPaths;
import stabled.client.discord.PresenceManager;
import stabled.client.hud.FpsHud;
import stabled.client.perf.PerformancePresets;
import stabled.client.render.MotionBlurRenderer;
import stabled.client.theme.ThemeManager;
import stabled.client.ui.screen.IntroScreen;
import stabled.client.ui.screen.StabledSettingsScreen;

/** Stabled Client entry point. */
public final class StabledClient implements ClientModInitializer {
	public static final String MOD_ID = "stabled";
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");
	private static final KeyMapping.Category CATEGORY = KeyMapping.Category.register(Identifier.fromNamespaceAndPath(MOD_ID, "main"));

	private static ClientSettings settings;
	private static Onboarding onboarding;
	private static String saveName = "Stabled";
	private static boolean ready;
	private static boolean introShownThisSession;
	private static boolean introOpen;
	private static boolean runtimeDirty = true;
	private static KeyMapping openSettingsKey;
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

		HudElementRegistry.addLast(Identifier.fromNamespaceAndPath(MOD_ID, "fps"), FpsHud::extract);
		openSettingsKey = KeyMappingHelper.registerKeyMapping(
			new KeyMapping("key.stabled.settings", InputConstants.KEY_RSHIFT, CATEGORY));

		PresenceManager.get().start(settings.discordEnabled, settings.discordShowElapsed);
		ClientLifecycleEvents.CLIENT_STARTED.register(client -> applyPresetIfNeeded());
		ClientTickEvents.END_CLIENT_TICK.register(StabledClient::onTick);
		ClientLifecycleEvents.CLIENT_STOPPING.register(client -> {
			PresenceManager.get().shutdown();
			RuntimeStatus.write("stopped", null, MotionBlurRenderer.unavailableReason(), false);
		});
		LOGGER.info("Stabled Client ready (launched by Stabled: {})", StabledPaths.launchedByStabled());
	}

	public static boolean isReady() {
		return ready;
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
			PerformancePresets.apply(settings.performancePreset);
			settings.performanceApplied = settings.performancePreset;
			settings.save();
		}
	}

	private static void onTick(Minecraft mc) {
		while (openSettingsKey.consumeClick()) {
			if (mc.gui.screen() == null) {
				mc.gui.setScreen(new StabledSettingsScreen(null));
			}
		}
		if (!introShownThisSession && !onboarding.completed && mc.gui.screen() instanceof TitleScreen title) {
			introShownThisSession = true;
			mc.gui.setScreen(new IntroScreen(title));
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
				MotionBlurRenderer.unavailableReason(), introOpen);
		}
	}

	private static PresenceManager.Activity activityFor(Minecraft mc) {
		if (mc.level == null) {
			return PresenceManager.Activity.MENUS;
		}
		if (mc.hasSingleplayerServer()) {
			return PresenceManager.Activity.SINGLEPLAYER;
		}
		ServerData server = mc.getCurrentServer();
		return server != null && server.isRealm() ? PresenceManager.Activity.REALMS : PresenceManager.Activity.MULTIPLAYER;
	}
}
