package stabled.client.config;

import com.google.gson.JsonObject;
import java.util.List;

/**
 * Per-Save Stabled client settings, stored in {@code <saveDir>/stabled/client.json}.
 * The launcher creates this file with the documented defaults when a Save is created and edits it while the
 * Save is not running; the game edits it while running. Schema: docs/CONTRACTS.md.
 */
public final class ClientSettings {
	public static final List<String> FPS_POSITIONS = List.of("top-left", "top-right", "bottom-left", "bottom-right");
	public static final List<String> FPS_COLORS = List.of("theme", "white", "green", "yellow", "cyan");
	public static final List<String> FPS_BACKGROUNDS = List.of("panel", "shadow", "none");
	public static final List<Double> FPS_SCALES = List.of(0.75, 1.0, 1.25, 1.5, 2.0);
	public static final List<String> PRESETS = List.of("performance", "balanced", "quality", "custom");
	public static final int MIN_BLUR = 10;
	public static final int MAX_BLUR = 100;

	private JsonObject raw = new JsonObject();

	// FPS display
	public boolean fpsEnabled = true;
	public String fpsPosition = "top-left";
	public double fpsScale = 1.0;
	public String fpsColor = "theme";
	public String fpsBackground = "panel";

	// Motion blur
	public boolean motionBlurEnabled = false;
	public int motionBlurStrength = 40;

	// Discord Rich Presence (on by default for every Save)
	public boolean discordEnabled = true;
	public boolean discordShowElapsed = true;

	// Performance
	public String performancePreset = "balanced";
	/** Last preset the game applied to options.txt, so presets are not re-applied over manual tweaks. */
	public String performanceApplied;

	// Menus
	public boolean styledMenus = true;

	public static ClientSettings load() {
		JsonObject o = JsonFiles.readObject(StabledPaths.clientSettingsFile());
		ClientSettings s = new ClientSettings();
		s.raw = o;
		JsonObject fps = JsonFiles.getObject(o, "fps");
		s.fpsEnabled = JsonFiles.getBool(fps, "enabled", s.fpsEnabled);
		s.fpsPosition = oneOf(JsonFiles.getString(fps, "position", s.fpsPosition), FPS_POSITIONS, "top-left");
		s.fpsScale = nearestScale(JsonFiles.getNumber(fps, "scale", s.fpsScale));
		s.fpsColor = oneOf(JsonFiles.getString(fps, "color", s.fpsColor), FPS_COLORS, "theme");
		s.fpsBackground = oneOf(JsonFiles.getString(fps, "background", s.fpsBackground), FPS_BACKGROUNDS, "panel");

		JsonObject blur = JsonFiles.getObject(o, "motionBlur");
		s.motionBlurEnabled = JsonFiles.getBool(blur, "enabled", s.motionBlurEnabled);
		s.motionBlurStrength = Appearance.clamp((int) Math.round(JsonFiles.getNumber(blur, "strength", s.motionBlurStrength)), MIN_BLUR, MAX_BLUR);

		JsonObject discord = JsonFiles.getObject(o, "discord");
		s.discordEnabled = JsonFiles.getBool(discord, "enabled", s.discordEnabled);
		s.discordShowElapsed = JsonFiles.getBool(discord, "showElapsed", s.discordShowElapsed);

		JsonObject perf = JsonFiles.getObject(o, "performance");
		s.performancePreset = oneOf(JsonFiles.getString(perf, "preset", s.performancePreset), PRESETS, "balanced");
		s.performanceApplied = JsonFiles.getString(perf, "applied", null);

		JsonObject menus = JsonFiles.getObject(o, "menus");
		s.styledMenus = JsonFiles.getBool(menus, "styled", s.styledMenus);
		return s;
	}

	public boolean save() {
		JsonObject o = this.raw;
		o.addProperty("version", 1);
		JsonObject fps = JsonFiles.getObject(o, "fps");
		fps.addProperty("enabled", this.fpsEnabled);
		fps.addProperty("position", this.fpsPosition);
		fps.addProperty("scale", this.fpsScale);
		fps.addProperty("color", this.fpsColor);
		fps.addProperty("background", this.fpsBackground);
		JsonObject blur = JsonFiles.getObject(o, "motionBlur");
		blur.addProperty("enabled", this.motionBlurEnabled);
		blur.addProperty("strength", this.motionBlurStrength);
		JsonObject discord = JsonFiles.getObject(o, "discord");
		discord.addProperty("enabled", this.discordEnabled);
		discord.addProperty("showElapsed", this.discordShowElapsed);
		JsonObject perf = JsonFiles.getObject(o, "performance");
		perf.addProperty("preset", this.performancePreset);
		if (this.performanceApplied != null) {
			perf.addProperty("applied", this.performanceApplied);
		}
		JsonObject menus = JsonFiles.getObject(o, "menus");
		menus.addProperty("styled", this.styledMenus);
		return JsonFiles.writeObject(StabledPaths.clientSettingsFile(), o);
	}

	private static String oneOf(String value, List<String> allowed, String fallback) {
		return allowed.contains(value) ? value : fallback;
	}

	private static double nearestScale(double v) {
		double best = 1.0;
		for (double s : FPS_SCALES) {
			if (Math.abs(s - v) < Math.abs(best - v)) {
				best = s;
			}
		}
		return best;
	}
}
