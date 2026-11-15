package stabled.client.theme;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.blaze3d.platform.NativeImage;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.function.Consumer;
import javax.imageio.ImageIO;
import net.minecraft.client.Minecraft;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import stabled.client.config.Appearance;
import stabled.client.config.StabledPaths;
import stabled.client.platform.Platform;

/**
 * Stabled's pre-themes in game: the background gallery that ships with the launcher, grouped by category.
 *
 * <p>The launcher passes its gallery folder as {@code -Dstabled.galleryDir}: {@code index.json} plus
 * {@code <category>--<image>.jpg} files. Minecraft textures must be PNG, so images are decoded with ImageIO on a
 * background thread and turned into small textures: a sharp thumbnail and a blurred "peek" used on category cards.
 * Choosing an image writes it as the Custom Background ({@code backgrounds/custom.png}), exactly as the launcher
 * does, so the launcher and every Save show it.
 */
public final class Gallery {
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");
	private static final int THUMB_W = 320;
	private static final int PEEK_W = 192;
	private static final int MAX_EDGE = 1920;

	public record Category(String id, String name, List<String> images) {
	}

	/** A loaded texture and its pixel size. */
	public record Tex(String id, int width, int height) {
	}

	private static List<Category> categories;
	private static Path dir;
	private static final Map<String, Tex> TEXTURES = new ConcurrentHashMap<>();
	private static final Map<String, Boolean> REQUESTED = new ConcurrentHashMap<>();
	private static final ExecutorService WORKER = Executors.newSingleThreadExecutor(r -> {
		Thread t = new Thread(r, "Stabled gallery");
		t.setDaemon(true);
		return t;
	});
	private static volatile String applying;

	private Gallery() {
	}

	/** Categories from the launcher's gallery, or an empty list when the game wasn't started by Stabled. */
	public static synchronized List<Category> categories() {
		if (categories == null) {
			categories = load();
		}
		return categories;
	}

	public static boolean available() {
		return !categories().isEmpty();
	}

	private static List<Category> load() {
		String prop = System.getProperty("stabled.galleryDir");
		if (prop == null || prop.isBlank()) {
			return List.of();
		}
		Path folder = Paths.get(prop);
		Path index = folder.resolve("index.json");
		if (!Files.isRegularFile(index)) {
			LOGGER.info("Stabled: no background gallery at {}", folder);
			return List.of();
		}
		try {
			JsonObject root = JsonParser.parseString(Files.readString(index, StandardCharsets.UTF_8)).getAsJsonObject();
			List<Category> out = new ArrayList<>();
			for (JsonElement e : root.getAsJsonArray("categories")) {
				JsonObject c = e.getAsJsonObject();
				String id = c.get("id").getAsString();
				if (!safe(id)) {
					continue;
				}
				List<String> images = new ArrayList<>();
				JsonArray arr = c.getAsJsonArray("images");
				for (JsonElement i : arr) {
					String img = i.getAsJsonObject().get("id").getAsString();
					if (safe(img) && Files.isRegularFile(folder.resolve(id + "--" + img + ".jpg"))) {
						images.add(img);
					}
				}
				if (!images.isEmpty()) {
					out.add(new Category(id, c.get("name").getAsString(), Collections.unmodifiableList(images)));
				}
			}
			dir = folder;
			return Collections.unmodifiableList(out);
		} catch (Exception e) {
			LOGGER.warn("Stabled: the background gallery index could not be read", e);
			return List.of();
		}
	}

	/** Ids come from the launcher's index, but they become file names and texture ids, so only allow plain slugs. */
	private static boolean safe(String id) {
		return id != null && id.matches("[a-z0-9][a-z0-9-]{0,63}");
	}

	public static String key(String category, String image) {
		return category + "/" + image;
	}

	/** The "category/image" currently used as the Custom Background, if it came from the gallery. */
	public static String selectedKey() {
		Appearance a = ThemeManager.appearance();
		if (!ThemeManager.CUSTOM_ID.equals(a.theme) || a.customBackground == null || a.customBackground.source == null) {
			return null;
		}
		String s = a.customBackground.source;
		return s.startsWith("gallery:") ? s.substring("gallery:".length()) : null;
	}

	/** Gallery key currently being applied (decoding and saving takes a moment), or null. */
	public static String applying() {
		return applying;
	}

	/**
	 * A sharp thumbnail ({@code blurred = false}) or a blurred peek, or null while it is still being prepared.
	 * The first call starts loading it in the background.
	 */
	public static Tex texture(String category, String image, boolean blurred) {
		String texId = "stabled:gallery/" + category + "/" + image + (blurred ? "_peek" : "_thumb");
		Tex tex = TEXTURES.get(texId);
		if (tex != null || dir == null) {
			return tex;
		}
		if (REQUESTED.putIfAbsent(texId, Boolean.TRUE) == null) {
			Path file = dir.resolve(category + "--" + image + ".jpg");
			WORKER.execute(() -> {
				try {
					BufferedImage src = read(file);
					BufferedImage small = blurred ? blur(src, PEEK_W) : scaleToWidth(src, THUMB_W);
					byte[] png = png(small);
					int w = small.getWidth();
					int h = small.getHeight();
					Minecraft.getInstance().execute(() -> {
						try (InputStream in = new ByteArrayInputStream(png)) {
							Platform.loadTexture(texId, NativeImage.read(in));
							TEXTURES.put(texId, new Tex(texId, w, h));
						} catch (IOException e) {
							LOGGER.warn("Stabled: gallery thumbnail could not be uploaded", e);
						}
					});
				} catch (Exception e) {
					LOGGER.warn("Stabled: gallery image {} could not be read", file.getFileName(), e);
				}
			});
		}
		return null;
	}

	/**
	 * Makes a gallery image the Custom Background: writes {@code backgrounds/custom.png} and selects the Custom
	 * Background theme. {@code done} runs on the game thread when finished; {@code failed} gets a message.
	 */
	public static void apply(String category, String image, Runnable done, Consumer<String> failed) {
		if (dir == null || applying != null) {
			return;
		}
		String key = key(category, image);
		applying = key;
		Path file = dir.resolve(category + "--" + image + ".jpg");
		WORKER.execute(() -> {
			try {
				BufferedImage src = read(file);
				if (Math.max(src.getWidth(), src.getHeight()) > MAX_EDGE) {
					src = scaleToWidth(src, src.getWidth() >= src.getHeight() ? MAX_EDGE : src.getWidth() * MAX_EDGE / src.getHeight());
				}
				byte[] png = png(src);
				Path target = StabledPaths.resolveInHome("backgrounds/custom.png");
				if (target == null) {
					throw new IOException("Stabled's data folder isn't available");
				}
				Files.createDirectories(target.getParent());
				Path tmp = target.resolveSibling("custom.png.tmp");
				Files.write(tmp, png);
				Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
				Minecraft.getInstance().execute(() -> {
					Appearance a = ThemeManager.appearance();
					Appearance.CustomBackground bg = a.customBackground != null ? a.customBackground : new Appearance.CustomBackground();
					bg.file = "backgrounds/custom.png";
					bg.focusX = 0.5;
					bg.focusY = 0.5;
					bg.revision = bg.revision + 1;
					bg.source = "gallery:" + key;
					a.customBackground = bg;
					a.theme = ThemeManager.CUSTOM_ID;
					ThemeManager.saveAppearance();
					applying = null;
					done.run();
				});
			} catch (Exception e) {
				LOGGER.warn("Stabled: gallery background {} could not be applied", key, e);
				Minecraft.getInstance().execute(() -> {
					applying = null;
					failed.accept("That background couldn't be applied. Try again, or choose it in the Stabled launcher.");
				});
			}
		});
	}

	private static BufferedImage read(Path file) throws IOException {
		BufferedImage img;
		try (InputStream in = Files.newInputStream(file)) {
			img = ImageIO.read(in);
		}
		if (img == null) {
			throw new IOException("Unsupported image: " + file.getFileName());
		}
		return toRgb(img);
	}

	private static BufferedImage toRgb(BufferedImage img) {
		if (img.getType() == BufferedImage.TYPE_INT_RGB) {
			return img;
		}
		BufferedImage out = new BufferedImage(img.getWidth(), img.getHeight(), BufferedImage.TYPE_INT_RGB);
		Graphics2D g = out.createGraphics();
		g.drawImage(img, 0, 0, null);
		g.dispose();
		return out;
	}

	/** Smooth downscale by halving steps (plain bilinear in one step aliases badly). */
	private static BufferedImage scaleToWidth(BufferedImage src, int width) {
		BufferedImage cur = src;
		int targetH = Math.max(1, Math.round(src.getHeight() * (width / (float) src.getWidth())));
		while (cur.getWidth() / 2 >= width) {
			cur = scale(cur, cur.getWidth() / 2, Math.max(1, cur.getHeight() / 2));
		}
		return scale(cur, width, targetH);
	}

	private static BufferedImage scale(BufferedImage src, int w, int h) {
		BufferedImage out = new BufferedImage(w, h, BufferedImage.TYPE_INT_RGB);
		Graphics2D g = out.createGraphics();
		g.setRenderingHint(RenderingHints.KEY_INTERPOLATION, RenderingHints.VALUE_INTERPOLATION_BILINEAR);
		g.setRenderingHint(RenderingHints.KEY_RENDERING, RenderingHints.VALUE_RENDER_QUALITY);
		g.drawImage(src, 0, 0, w, h, null);
		g.dispose();
		return out;
	}

	/** A soft blur: shrink far down, then grow back in smooth steps. */
	private static BufferedImage blur(BufferedImage src, int width) {
		BufferedImage small = scaleToWidth(src, Math.max(8, width / 8));
		int h = Math.max(1, Math.round(src.getHeight() * (width / (float) src.getWidth())));
		BufferedImage mid = scale(small, width / 3, Math.max(1, h / 3));
		return scale(mid, width, h);
	}

	private static byte[] png(BufferedImage img) throws IOException {
		ByteArrayOutputStream out = new ByteArrayOutputStream();
		if (!ImageIO.write(img, "png", out)) {
			throw new IOException("PNG encoding is unavailable");
		}
		return out.toByteArray();
	}
}
