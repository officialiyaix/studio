package stabled.client.platform;

import net.minecraft.client.Minecraft;
import net.minecraft.client.Options;
//#if MC >= 1.21.11
import net.minecraft.client.GraphicsPreset;
//#else
import net.minecraft.client.CloudStatus;
import net.minecraft.client.GraphicsStatus;
	//#if MC >= 1.21.2
import net.minecraft.server.level.ParticleStatus;
	//#else
import net.minecraft.client.ParticleStatus;
	//#endif
//#endif

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
 * From Minecraft 1.21.11 the vanilla graphics presets are applied as they are. Older versions have no presets, only
 * the Fast/Fancy graphics mode, so Stabled sets the same core options the newer presets do: graphics mode, render
 * and simulation distance, smooth lighting, clouds, particles, entity shadows, biome blend and entity distance.
 * All of these apply immediately. Only the graphics API choice (OpenGL/Vulkan, Minecraft 26.2+) needs a restart.
 */
final class Presets {
	private Presets() {
	}

	static void apply(String preset) {
		Minecraft mc = Minecraft.getInstance();
		Options o = mc.options;
		switch (preset) {
			case "performance" -> applyVanilla(o, true);
			case "balanced" -> {
				applyVanilla(o, false);
				o.renderDistance().set(12);
				o.simulationDistance().set(8);
			}
			case "quality" -> applyVanilla(o, false);
			default -> {
				return;
			}
		}
		o.save();
	}

	private static void applyVanilla(Options o, boolean fast) {
//#if MC >= 1.21.11
		GraphicsPreset preset = fast ? GraphicsPreset.FAST : GraphicsPreset.FANCY;
		o.graphicsPreset().set(preset);
		// Re-apply even if the preset was already selected, so drifted values return to the preset.
		o.applyGraphicsPreset(preset);
//#else
		o.graphicsMode().set(fast ? GraphicsStatus.FAST : GraphicsStatus.FANCY);
		o.renderDistance().set(fast ? 8 : 16);
		o.simulationDistance().set(fast ? 6 : 12);
		o.ambientOcclusion().set(!fast);
		o.cloudStatus().set(fast ? CloudStatus.FAST : CloudStatus.FANCY);
		o.particles().set(fast ? ParticleStatus.DECREASED : ParticleStatus.ALL);
		o.entityShadows().set(!fast);
		o.biomeBlendRadius().set(fast ? 1 : 2);
		o.entityDistanceScaling().set(fast ? 0.75 : 1.0);
//#endif
	}
}
