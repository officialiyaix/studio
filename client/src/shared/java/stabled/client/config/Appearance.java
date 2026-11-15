package stabled.client.config;

import com.google.gson.JsonElement;
import com.google.gson.JsonNull;
import com.google.gson.JsonObject;

/**
 * Global appearance preferences (theme + surface style). Shared with the launcher through
 * {@code <stabled.home>/appearance.json}; see docs/CONTRACTS.md for the schema.
 *
 * <p>Unknown keys written by the launcher are preserved when the game saves.
 */
public final class Appearance {
	public static final String SURFACE_SIMPLE = "simple";
	public static final String SURFACE_GLASS = "glass";
	public static final int MIN_TRANSPARENCY = 10;
	public static final int MAX_TRANSPARENCY = 90;

	private JsonObject raw = new JsonObject();
	public String theme = "stabled-night";
	public String surface = SURFACE_SIMPLE;
	/** Higher = background more visible. 10% transparency = 90% surface opacity. */
	public int glassTransparency = 35;
	public CustomBackground customBackground;

	public static final class CustomBackground {
		public String file;
		public double focusX = 0.5;
		public double focusY = 0.5;
		public double dim = 0.45;
		public long revision;
		/** Where the image came from: "upload" or "gallery:<category>/<image>" (null if unknown). */
		public String source;
	}

	public boolean isGlass() {
		return SURFACE_GLASS.equals(this.surface);
	}

	/** Opacity applied to translucent surfaces (0..1). Simple mode is always fully opaque. */
	public float surfaceOpacity() {
		return this.isGlass() ? 1.0F - this.glassTransparency / 100.0F : 1.0F;
	}

	public static Appearance load() {
		JsonObject o = JsonFiles.readObject(StabledPaths.appearanceFile());
		Appearance a = new Appearance();
		a.raw = o;
		a.theme = JsonFiles.getString(o, "theme", a.theme);
		String surface = JsonFiles.getString(o, "surface", a.surface);
		a.surface = SURFACE_GLASS.equals(surface) ? SURFACE_GLASS : SURFACE_SIMPLE;
		a.glassTransparency = clamp((int) Math.round(JsonFiles.getNumber(o, "glassTransparency", a.glassTransparency)), MIN_TRANSPARENCY, MAX_TRANSPARENCY);
		JsonElement bg = o.get("customBackground");
		if (bg != null && bg.isJsonObject()) {
			JsonObject b = bg.getAsJsonObject();
			CustomBackground c = new CustomBackground();
			c.file = JsonFiles.getString(b, "file", null);
			c.focusX = clamp01(JsonFiles.getNumber(b, "focusX", 0.5));
			c.focusY = clamp01(JsonFiles.getNumber(b, "focusY", 0.5));
			c.dim = Math.max(0.0, Math.min(0.85, JsonFiles.getNumber(b, "dim", 0.45)));
			c.revision = (long) JsonFiles.getNumber(b, "revision", 0);
			c.source = JsonFiles.getString(b, "source", null);
			a.customBackground = c.file != null ? c : null;
		}
		return a;
	}

	public boolean save() {
		JsonObject o = this.raw;
		o.addProperty("version", 1);
		o.addProperty("theme", this.theme);
		o.addProperty("surface", this.surface);
		o.addProperty("glassTransparency", this.glassTransparency);
		if (this.customBackground == null) {
			o.add("customBackground", JsonNull.INSTANCE);
		} else {
			// Keep fields the launcher records (e.g. which gallery image this is) alongside the ones edited here.
			JsonElement existing = o.get("customBackground");
			JsonObject b = existing != null && existing.isJsonObject() ? existing.getAsJsonObject() : new JsonObject();
			b.addProperty("file", this.customBackground.file);
			b.addProperty("focusX", this.customBackground.focusX);
			b.addProperty("focusY", this.customBackground.focusY);
			b.addProperty("dim", this.customBackground.dim);
			b.addProperty("revision", this.customBackground.revision);
			if (this.customBackground.source != null) {
				b.addProperty("source", this.customBackground.source);
			}
			o.add("customBackground", b);
		}
		return JsonFiles.writeObject(StabledPaths.appearanceFile(), o);
	}

	static int clamp(int v, int min, int max) {
		return Math.max(min, Math.min(max, v));
	}

	static double clamp01(double v) {
		return Math.max(0.0, Math.min(1.0, v));
	}
}
