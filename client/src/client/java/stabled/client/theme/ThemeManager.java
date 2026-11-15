package stabled.client.theme;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.blaze3d.platform.NativeImage;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import stabled.client.config.Appearance;
import stabled.client.config.JsonFiles;
import stabled.client.config.StabledPaths;

/**
 * Holds the nine theme choices and the global appearance, and keeps them in sync with the launcher.
 * The appearance file is re-checked at most once per second, and only while Stabled UI is being drawn.
 */
public final class ThemeManager {
	public static final String CUSTOM_ID = "custom";
	public static final Identifier CUSTOM_BACKGROUND = Identifier.fromNamespaceAndPath("stabled", "dynamic/custom_background");
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");
	private static final long RECHECK_MS = 1000L;

	private static List<Theme> builtIns = List.of();
	private static Theme fallback;
	private static String customName = "Custom Background";
	private static String customDescription = "A user-uploaded image from the computer";
	private static double customDefaultDim = 0.45;
	private static Appearance appearance = new Appearance();
	private static long appearanceMtime = -1L;
	private static long lastCheck;

	private static long loadedBackgroundRevision = Long.MIN_VALUE;
	private static @Nullable String loadedBackgroundFile;
	private static boolean backgroundLoaded;
	private static int backgroundWidth;
	private static int backgroundHeight;
	private static @Nullable String backgroundError;

	private ThemeManager() {
	}

	public static void init() {
		try (InputStream in = ThemeManager.class.getClassLoader().getResourceAsStream("assets/stabled/themes.json")) {
			if (in == null) {
				throw new IllegalStateException("themes.json missing from the Stabled client jar");
			}
			JsonObject root = JsonParser.parseReader(new InputStreamReader(in, StandardCharsets.UTF_8)).getAsJsonObject();
			List<Theme> themes = new ArrayList<>();
			for (JsonElement e : root.getAsJsonArray("themes")) {
				themes.add(Theme.fromJson(e.getAsJsonObject()));
			}
			builtIns = Collections.unmodifiableList(themes);
			JsonObject custom = root.getAsJsonObject("custom");
			customName = custom.get("name").getAsString();
			customDescription = custom.get("description").getAsString();
			customDefaultDim = custom.get("defaultDim").getAsDouble();
			String base = custom.get("baseTheme").getAsString();
			fallback = builtIns.stream().filter(t -> t.id().equals(base)).findFirst().orElse(builtIns.getFirst());
		} catch (Exception e) {
			throw new IllegalStateException("Stabled could not load its themes", e);
		}
		reloadAppearance(true);
	}

	/** The eight built-in themes, in display order. */
	public static List<Theme> builtInThemes() {
		return builtIns;
	}

	public static String customName() {
		return customName;
	}

	public static String customDescription() {
		return customDescription;
	}

	public static Appearance appearance() {
		pollAppearance();
		return appearance;
	}

	/** Colours of the active theme. Custom Background uses the base theme's colours over the image. */
	public static Theme current() {
		pollAppearance();
		return resolve(appearance.theme);
	}

	public static Theme resolve(String id) {
		if (CUSTOM_ID.equals(id)) {
			return fallback.withIdentity(CUSTOM_ID, customName, customDescription);
		}
		for (Theme t : builtIns) {
			if (t.id().equals(id)) {
				return t;
			}
		}
		return fallback;
	}

	public static boolean isCustomSelected() {
		return CUSTOM_ID.equals(appearance().theme);
	}

	public static boolean hasCustomImage() {
		Appearance.CustomBackground bg = appearance().customBackground;
		return bg != null && bg.file != null;
	}

	public static void select(String themeId) {
		appearance.theme = themeId;
		saveAppearance();
	}

	public static void setSurface(String surface) {
		appearance.surface = surface;
		saveAppearance();
	}

	public static void setTransparency(int transparency, boolean persist) {
		appearance.glassTransparency = Math.max(Appearance.MIN_TRANSPARENCY, Math.min(Appearance.MAX_TRANSPARENCY, transparency));
		if (persist) {
			saveAppearance();
		}
	}

	public static void saveAppearance() {
		if (appearance.save()) {
			appearanceMtime = JsonFiles.modifiedTime(StabledPaths.appearanceFile());
		}
	}

	private static void pollAppearance() {
		long now = System.currentTimeMillis();
		if (now - lastCheck >= RECHECK_MS) {
			lastCheck = now;
			reloadAppearance(false);
		}
	}

	private static void reloadAppearance(boolean force) {
		long mtime = JsonFiles.modifiedTime(StabledPaths.appearanceFile());
		if (force || mtime != appearanceMtime) {
			appearanceMtime = mtime;
			appearance = Appearance.load();
		}
	}

	/**
	 * Returns the custom background texture, (re)loading it when the launcher has replaced the image.
	 * Returns null when no usable image exists, in which case callers draw the base theme instead.
	 */
	public static @Nullable Identifier customBackgroundTexture() {
		Appearance.CustomBackground bg = appearance().customBackground;
		if (bg == null || bg.file == null) {
			releaseBackground();
			backgroundError = null;
			return null;
		}
		if (bg.revision != loadedBackgroundRevision || !bg.file.equals(loadedBackgroundFile)) {
			loadedBackgroundRevision = bg.revision;
			loadedBackgroundFile = bg.file;
			loadBackground(bg.file);
		}
		return backgroundLoaded ? CUSTOM_BACKGROUND : null;
	}

	public static int backgroundWidth() {
		return backgroundWidth;
	}

	public static int backgroundHeight() {
		return backgroundHeight;
	}

	public static double customDim() {
		Appearance.CustomBackground bg = appearance().customBackground;
		return bg != null ? bg.dim : customDefaultDim;
	}

	public static @Nullable String backgroundError() {
		return backgroundError;
	}

	private static void loadBackground(String relative) {
		releaseBackground();
		Path file = StabledPaths.resolveInHome(relative);
		if (file == null || !Files.isRegularFile(file)) {
			backgroundError = "The custom background image could not be found. Choose it again in the Stabled launcher.";
			return;
		}
		try (InputStream in = Files.newInputStream(file)) {
			NativeImage image = NativeImage.read(in);
			backgroundWidth = image.getWidth();
			backgroundHeight = image.getHeight();
			Minecraft.getInstance().getTextureManager().register(CUSTOM_BACKGROUND, new DynamicTexture(() -> "Stabled custom background", image));
			backgroundLoaded = true;
			backgroundError = null;
		} catch (Exception e) {
			LOGGER.warn("Stabled: custom background could not be loaded", e);
			backgroundError = "The custom background image could not be read. Choose it again in the Stabled launcher.";
		}
	}

	private static void releaseBackground() {
		if (backgroundLoaded) {
			Minecraft.getInstance().getTextureManager().release(CUSTOM_BACKGROUND);
			backgroundLoaded = false;
		}
	}
}
