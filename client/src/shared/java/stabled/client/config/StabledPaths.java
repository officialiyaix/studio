package stabled.client.config;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import net.fabricmc.loader.api.FabricLoader;

/**
 * Resolves the files shared between the Stabled launcher and the in-game client.
 *
 * <p>The launcher passes two JVM system properties when it starts a Save:
 * <ul>
 *   <li>{@code stabled.home} – the launcher data root (global appearance, managed background image).</li>
 *   <li>{@code stabled.saveDir} – the Save's root directory (per-Save Stabled settings and onboarding state).</li>
 * </ul>
 * When the client runs outside the launcher (for example {@code gradlew runClient}), both fall back to
 * folders beside the game directory so that development never touches a real installation.
 */
public final class StabledPaths {
	private static Path home;
	private static Path saveDir;
	private static boolean launchedByStabled;

	private StabledPaths() {
	}

	public static synchronized void init() {
		Path gameDir = FabricLoader.getInstance().getGameDir();
		String homeProp = System.getProperty("stabled.home");
		String saveProp = System.getProperty("stabled.saveDir");
		launchedByStabled = homeProp != null && saveProp != null;
		home = homeProp != null ? Paths.get(homeProp) : gameDir.resolve("stabled-dev-home");
		saveDir = saveProp != null ? Paths.get(saveProp) : gameDir.resolve("stabled-dev-save");
		try {
			Files.createDirectories(stabledDir());
		} catch (Exception ignored) {
			// Reported later when a write actually fails.
		}
	}

	public static boolean launchedByStabled() {
		return launchedByStabled;
	}

	public static Path home() {
		return home;
	}

	public static Path saveDir() {
		return saveDir;
	}

	/** Per-Save folder that holds Stabled's own state, kept outside the Minecraft game directory. */
	public static Path stabledDir() {
		return saveDir.resolve("stabled");
	}

	public static Path appearanceFile() {
		return home.resolve("appearance.json");
	}

	public static Path clientSettingsFile() {
		return stabledDir().resolve("client.json");
	}

	public static Path onboardingFile() {
		return stabledDir().resolve("onboarding.json");
	}

	/** Written by the game so the launcher can show accurate live integration status. */
	public static Path runtimeFile() {
		return stabledDir().resolve("runtime.json");
	}

	public static Path resolveInHome(String relative) {
		Path resolved = home.resolve(relative).normalize();
		// Never follow a path that escapes the managed launcher folder.
		return resolved.startsWith(home.normalize()) ? resolved : null;
	}
}
