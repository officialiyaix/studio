package stabled.client.theme;

import com.google.gson.JsonObject;

/** One built-in theme from shared/themes.json. Colours are opaque ARGB. */
public record Theme(
	String id,
	String name,
	String description,
	boolean light,
	int bg,
	int bgAlt,
	int surface,
	int surfaceAlt,
	int surfaceHover,
	int border,
	int text,
	int textMuted,
	int accent,
	int accentAlt,
	int accentText,
	int cta,
	int ctaText,
	int success,
	int warning,
	int danger,
	int info
) {
	static Theme fromJson(JsonObject o) {
		JsonObject c = o.getAsJsonObject("colors");
		return new Theme(
			o.get("id").getAsString(),
			o.get("name").getAsString(),
			o.get("description").getAsString(),
			o.has("light") && o.get("light").getAsBoolean(),
			hex(c, "bg"), hex(c, "bgAlt"), hex(c, "surface"), hex(c, "surfaceAlt"), hex(c, "surfaceHover"),
			hex(c, "border"), hex(c, "text"), hex(c, "textMuted"), hex(c, "accent"), hex(c, "accentAlt"),
			hex(c, "accentText"), hex(c, "cta"), hex(c, "ctaText"), hex(c, "success"), hex(c, "warning"),
			hex(c, "danger"), hex(c, "info")
		);
	}

	Theme withIdentity(String id, String name, String description) {
		return new Theme(id, name, description, this.light, this.bg, this.bgAlt, this.surface, this.surfaceAlt, this.surfaceHover,
			this.border, this.text, this.textMuted, this.accent, this.accentAlt, this.accentText, this.cta, this.ctaText,
			this.success, this.warning, this.danger, this.info);
	}

	private static int hex(JsonObject c, String key) {
		String v = c.get(key).getAsString().trim();
		if (v.startsWith("#")) {
			v = v.substring(1);
		}
		return 0xFF000000 | Integer.parseInt(v, 16);
	}
}
