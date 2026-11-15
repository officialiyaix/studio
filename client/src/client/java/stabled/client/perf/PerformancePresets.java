package stabled.client.perf;

import net.minecraft.client.GraphicsPreset;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Options;
import net.minecraft.network.chat.Component;

/**
 * Stabled performance presets. Each preset is a documented set of vanilla video options; the performance mods
 * (Sodium, Lithium, …) installed by the launcher do their work regardless of the preset.
 *
 * <ul>
 *   <li><b>Performance</b> – vanilla "Fast" graphics (render distance 8, simulation 6).</li>
 *   <li><b>Balanced</b> – vanilla "Fancy" graphics with render distance 12 and simulation distance 8.</li>
 *   <li><b>Quality</b> – vanilla "Fancy" graphics (render distance 16, simulation 12).</li>
 *   <li><b>Custom</b> – Stabled leaves every video option exactly as you set it.</li>
 * </ul>
 * All of these apply immediately. Only the graphics API choice (OpenGL/Vulkan) needs a game restart.
 */
public final class PerformancePresets {
	private PerformancePresets() {
	}

	public static Component label(String preset) {
		return Component.translatable("stabled.preset." + preset);
	}

	public static Component description(String preset) {
		return Component.translatable("stabled.preset." + preset + ".desc");
	}

	public static void apply(String preset) {
		Minecraft mc = Minecraft.getInstance();
		Options o = mc.options;
		switch (preset) {
			case "performance" -> applyVanilla(o, GraphicsPreset.FAST);
			case "balanced" -> {
				applyVanilla(o, GraphicsPreset.FANCY);
				o.renderDistance().set(12);
				o.simulationDistance().set(8);
			}
			case "quality" -> applyVanilla(o, GraphicsPreset.FANCY);
			default -> {
				return;
			}
		}
		o.save();
	}

	private static void applyVanilla(Options o, GraphicsPreset preset) {
		o.graphicsPreset().set(preset);
		// Re-apply even if the preset was already selected, so drifted values return to the preset.
		o.applyGraphicsPreset(preset);
	}
}
