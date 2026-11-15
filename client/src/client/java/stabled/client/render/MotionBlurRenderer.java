package stabled.client.render;

import com.mojang.blaze3d.buffers.Std140Builder;
import com.mojang.blaze3d.pipeline.RenderTarget;
import com.mojang.blaze3d.pipeline.TextureTarget;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.renderpearl.api.GpuFormat;
import com.mojang.renderpearl.api.buffers.GpuBuffer;
import com.mojang.renderpearl.api.buffers.GpuBufferSlice;
import com.mojang.renderpearl.api.commands.RenderPass;
import com.mojang.renderpearl.api.pipeline.BindGroupLayout;
import com.mojang.renderpearl.api.pipeline.ColorTargetState;
import com.mojang.renderpearl.api.pipeline.CompiledRenderPipeline;
import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import com.mojang.renderpearl.api.pipeline.UniformType;
import com.mojang.renderpearl.api.textures.FilterMode;
import java.lang.reflect.Method;
import java.util.Optional;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.MappableRingBuffer;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import stabled.client.StabledClient;
import stabled.client.config.ClientSettings;

/**
 * Real motion blur in the game's render pipeline: frame accumulation with exponential, frame-rate independent
 * decay. It runs after the level (and vanilla post effects) are rendered into the main target and before the GUI,
 * so the HUD, chat and menus stay sharp.
 *
 * <p>Resources (two screen-sized colour targets and a small uniform ring buffer) exist only while the effect is
 * enabled and are released as soon as it is turned off, the world is left, or the game stops.
 */
public final class MotionBlurRenderer {
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");
	private static final RenderPipeline PIPELINE = RenderPipeline.builder(RenderPipelines.POST_PROCESSING_SNIPPET)
		.withLocation(Identifier.fromNamespaceAndPath("stabled", "pipeline/motion_blur"))
		.withVertexShader(Identifier.withDefaultNamespace("core/screenquad"))
		.withFragmentShader(Identifier.fromNamespaceAndPath("stabled", "post/motion_blur"))
		.withBindGroupLayout(BindGroupLayout.builder()
			.withUniform("InSampler", UniformType.COMBINED_IMAGE_SAMPLER)
			.withUniform("PrevSampler", UniformType.COMBINED_IMAGE_SAMPLER)
			.withUniform("MotionBlurConfig", UniformType.UNIFORM_BUFFER)
			.build())
		.withColorTargetState(ColorTargetState.DEFAULT)
		.build();
	/** History weight per 1/60 s at 100% strength. Kept below 1 so trails always decay. */
	private static final float MAX_RETENTION = 0.88F;

	private static @Nullable TextureTarget history;
	private static @Nullable TextureTarget scratch;
	private static @Nullable MappableRingBuffer config;
	private static boolean needsSeed = true;
	private static long lastFrameNanos;
	private static @Nullable ClientLevel lastLevel;
	private static @Nullable String failure;
	private static boolean active;

	private static boolean irisChecked;
	private static @Nullable Method irisInstance;
	private static @Nullable Method irisShaderPackInUse;

	private MotionBlurRenderer() {
	}

	/** Called by GameRendererMixin after world post effects, before the GUI. Render thread only. */
	public static void afterLevel(RenderTarget main) {
		ClientSettings settings = StabledClient.settings();
		Minecraft mc = Minecraft.getInstance();
		if (!settings.motionBlurEnabled || failure != null || unavailableReason() != null || mc.level == null) {
			release();
			return;
		}
		try {
			CompiledRenderPipeline compiled = RenderSystem.getCompiledPipelineNullable(PIPELINE);
			if (compiled == null) {
				fail("The motion blur shader could not be compiled for this renderer.");
				return;
			}
			if (mc.level != lastLevel) {
				lastLevel = mc.level;
				needsSeed = true;
			}
			ensureTargets(main.width, main.height);
			long now = System.nanoTime();
			if (needsSeed) {
				// Seed history with the current frame so enabling, resizing or changing worlds never flashes black.
				history.copyColorFrom(main);
				needsSeed = false;
				lastFrameNanos = now;
				active = true;
				return;
			}
			double dt = Math.min(0.25, Math.max(1.0 / 1000.0, (now - lastFrameNanos) / 1.0E9));
			lastFrameNanos = now;
			float retention = MAX_RETENTION * (settings.motionBlurStrength / 100F);
			float weight = (float) Math.pow(retention, dt * 60.0);

			GpuBuffer ubo = config.currentBuffer();
			try (GpuBufferSlice.MappedView view = ubo.map(false, true)) {
				Std140Builder.intoBuffer(view.data()).putVec4(weight, 0F, 0F, 0F);
			}
			try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder()
				.createRenderPass(() -> "Stabled motion blur", scratch.getColorTextureView(), Optional.empty())) {
				pass.setPipeline(compiled);
				RenderSystem.bindDefaultUniforms(pass);
				pass.setUniform("InSampler", main.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
				pass.setUniform("PrevSampler", history.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
				pass.setUniform("MotionBlurConfig", ubo);
				pass.draw(3, 1, 0, 0);
			}
			config.rotate();
			main.copyColorFrom(scratch);
			TextureTarget swap = history;
			history = scratch;
			scratch = swap;
			active = true;
		} catch (RuntimeException e) {
			LOGGER.warn("Stabled: motion blur disabled after a rendering error", e);
			fail("Motion blur stopped after a rendering error. Toggle it off and on to retry.");
		}
	}

	private static void ensureTargets(int width, int height) {
		if (history == null || history.width != width || history.height != height) {
			destroyTargets();
			history = new TextureTarget("Stabled motion blur history", width, height, GpuFormat.RGBA8_UNORM, null);
			scratch = new TextureTarget("Stabled motion blur output", width, height, GpuFormat.RGBA8_UNORM, null);
			needsSeed = true;
		}
		if (config == null) {
			config = new MappableRingBuffer(() -> "Stabled motion blur config", GpuBuffer.USAGE_UNIFORM | GpuBuffer.USAGE_MAP_WRITE, 16);
		}
	}

	private static void fail(String reason) {
		failure = reason;
		release();
	}

	/** Releases GPU resources. Safe to call every frame. Render thread only. */
	public static void release() {
		if (history == null && scratch == null && config == null) {
			active = false;
			return;
		}
		destroyTargets();
		if (config != null) {
			config.close();
			config = null;
		}
		needsSeed = true;
		lastLevel = null;
		active = false;
	}

	private static void destroyTargets() {
		if (history != null) {
			history.destroyBuffers();
			history = null;
		}
		if (scratch != null) {
			scratch.destroyBuffers();
			scratch = null;
		}
	}

	/** Clears a previous failure so the effect can be retried (used when the toggle is switched on again). */
	public static void resetFailure() {
		failure = null;
	}

	public static boolean isActive() {
		return active;
	}

	/** Non-null when the effect cannot run in the current configuration; shown next to the control. */
	public static @Nullable String unavailableReason() {
		if (failure != null) {
			return failure;
		}
		if (irisShaderPackActive()) {
			return "Not available while an Iris shader pack is active. Shader packs replace the renderer that motion blur runs in.";
		}
		return null;
	}

	private static boolean irisShaderPackActive() {
		if (!irisChecked) {
			irisChecked = true;
			if (FabricLoader.getInstance().isModLoaded("iris")) {
				try {
					Class<?> api = Class.forName("net.irisshaders.iris.api.v0.IrisApi");
					irisInstance = api.getMethod("getInstance");
					irisShaderPackInUse = api.getMethod("isShaderPackInUse");
				} catch (ReflectiveOperationException e) {
					LOGGER.info("Stabled: Iris is installed but its API was not found; assuming no shader pack");
				}
			}
		}
		if (irisInstance == null || irisShaderPackInUse == null) {
			return false;
		}
		try {
			return (Boolean) irisShaderPackInUse.invoke(irisInstance.invoke(null));
		} catch (ReflectiveOperationException | ClassCastException e) {
			return false;
		}
	}
}
