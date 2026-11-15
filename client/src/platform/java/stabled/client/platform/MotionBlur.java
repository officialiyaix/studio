package stabled.client.platform;

import com.mojang.blaze3d.pipeline.RenderTarget;
import com.mojang.blaze3d.pipeline.TextureTarget;
import java.lang.reflect.Method;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import stabled.client.StabledClient;
import stabled.client.config.ClientSettings;
//#if MC >= 26.3
import com.mojang.blaze3d.buffers.Std140Builder;
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
import java.util.Optional;
import net.minecraft.client.renderer.MappableRingBuffer;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.resources.Identifier;
//#elif MC >= 26.2
import com.mojang.blaze3d.GpuFormat;
import com.mojang.blaze3d.buffers.GpuBuffer;
import com.mojang.blaze3d.buffers.GpuBufferSlice;
import com.mojang.blaze3d.buffers.Std140Builder;
import com.mojang.blaze3d.pipeline.BindGroupLayout;
import com.mojang.blaze3d.pipeline.ColorTargetState;
import com.mojang.blaze3d.pipeline.RenderPipeline;
import com.mojang.blaze3d.shaders.UniformType;
import com.mojang.blaze3d.systems.RenderPass;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.blaze3d.textures.FilterMode;
import java.util.Optional;
import net.minecraft.client.renderer.MappableRingBuffer;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.resources.Identifier;
//#elif MC >= 1.21.5
import com.mojang.blaze3d.pipeline.BlendFunction;
import com.mojang.blaze3d.pipeline.RenderPipeline;
import com.mojang.blaze3d.platform.DestFactor;
import com.mojang.blaze3d.platform.SourceFactor;
import com.mojang.blaze3d.systems.RenderPass;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.blaze3d.vertex.DefaultVertexFormat;
import com.mojang.blaze3d.vertex.VertexFormat;
import java.util.OptionalInt;
import org.lwjgl.opengl.GL14;
	//#if MC >= 26.1
import com.mojang.blaze3d.pipeline.ColorTargetState;
import java.util.Optional;
	//#else
import com.mojang.blaze3d.platform.DepthTestFunction;
	//#endif
	//#if MC >= 1.21.11
import com.mojang.blaze3d.textures.FilterMode;
	//#elif MC < 1.21.9
import com.mojang.blaze3d.buffers.GpuBuffer;
	//#endif
//#else
import com.mojang.blaze3d.platform.GlStateManager;
import com.mojang.blaze3d.systems.RenderSystem;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL14;
import org.lwjgl.opengl.GL30;
	//#if MC < 1.21
import stabled.client.mixin.RenderTargetInvoker;
	//#endif
//#endif

/**
 * Real motion blur in the game's render pipeline: frame accumulation with exponential, frame-rate independent
 * decay. It runs after the level (and vanilla post effects) are rendered into the main target and before the GUI,
 * so the HUD, chat and menus stay sharp.
 *
 * <p>Each frame the accumulated history is mixed into the new frame ({@code out = mix(frame, history, w)}) and the
 * result becomes the next history. On OpenGL-only versions (up to 26.1) the mix is a blended copy that uses the GL
 * constant blend colour as {@code w}; on 26.2 and newer, which can also run on Vulkan, it is a small shader pass.
 *
 * <p>Resources (screen-sized colour targets, and on 26.2+ a small uniform ring buffer) exist only while the effect
 * is enabled and are released as soon as it is turned off, the world is left, or the game stops.
 */
public final class MotionBlur {
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");
	/** History weight per 1/60 s at 100% strength. Kept below 1 so trails always decay. */
	private static final float MAX_RETENTION = 0.88F;

	private static TextureTarget history;
	private static boolean needsSeed = true;
	private static long lastFrameNanos;
	private static ClientLevel lastLevel;
	private static String failure;
	private static boolean active;

	private static boolean irisChecked;
	private static Method irisInstance;
	private static Method irisShaderPackInUse;

//#if MC >= 26.2
	private static final RenderPipeline PIPELINE = RenderPipeline.builder(RenderPipelines.POST_PROCESSING_SNIPPET)
		.withLocation(Identifier.fromNamespaceAndPath("stabled", "pipeline/motion_blur"))
		.withVertexShader(Identifier.withDefaultNamespace("core/screenquad"))
		.withFragmentShader(Identifier.fromNamespaceAndPath("stabled", "post/motion_blur"))
	//#if MC >= 26.3
		.withBindGroupLayout(BindGroupLayout.builder()
			.withUniform("InSampler", UniformType.COMBINED_IMAGE_SAMPLER)
			.withUniform("PrevSampler", UniformType.COMBINED_IMAGE_SAMPLER)
			.withUniform("MotionBlurConfig", UniformType.UNIFORM_BUFFER)
			.build())
	//#else
		.withBindGroupLayout(BindGroupLayout.builder()
			.withSampler("InSampler")
			.withSampler("PrevSampler")
			.withUniform("MotionBlurConfig", UniformType.UNIFORM_BUFFER)
			.build())
	//#endif
		.withColorTargetState(ColorTargetState.DEFAULT)
		.build();
	/** Output of the shader pass; swapped with {@link #history} every frame. */
	private static TextureTarget scratch;
	private static MappableRingBuffer config;
//#elif MC >= 1.21.5
	/** The vanilla screen blit, blended with the GL constant alpha (the history weight). */
	private static final RenderPipeline PIPELINE = RenderPipeline.builder()
		.withLocation(Gfx.id("stabled:pipeline/motion_blur"))
	//#if MC >= 1.21.9
		.withVertexShader("core/screenquad")
	//#else
		.withVertexShader("core/blit_screen")
	//#endif
		.withFragmentShader("core/blit_screen")
		.withSampler("InSampler")
	//#if MC >= 26.1
		.withColorTargetState(new ColorTargetState(Optional.of(new BlendFunction(SourceFactor.CONSTANT_ALPHA, DestFactor.ONE_MINUS_CONSTANT_ALPHA)), 7))
	//#else
		.withBlend(new BlendFunction(SourceFactor.CONSTANT_ALPHA, DestFactor.ONE_MINUS_CONSTANT_ALPHA))
		.withDepthWrite(false)
		.withDepthTestFunction(DepthTestFunction.NO_DEPTH_TEST)
		.withColorWrite(true, false)
	//#endif
	//#if MC >= 1.21.9
		.withVertexFormat(DefaultVertexFormat.EMPTY, VertexFormat.Mode.TRIANGLES)
	//#else
		.withVertexFormat(DefaultVertexFormat.POSITION, VertexFormat.Mode.QUADS)
	//#endif
		.build();
//#endif

	private MotionBlur() {
	}

	/** Called by GameRendererMixin after the level and world post effects, before the GUI. Render thread only. */
	public static void afterLevel(RenderTarget main) {
		ClientSettings settings = StabledClient.settings();
		Minecraft mc = Minecraft.getInstance();
		if (!settings.motionBlurEnabled || failure != null || unavailableReason() != null || mc.level == null) {
			release();
			return;
		}
		try {
			if (!pipelineReady()) {
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
				copy(main, history);
				needsSeed = false;
				lastFrameNanos = now;
				active = true;
				return;
			}
			double dt = Math.min(0.25, Math.max(1.0 / 1000.0, (now - lastFrameNanos) / 1.0E9));
			lastFrameNanos = now;
			float retention = MAX_RETENTION * (settings.motionBlurStrength / 100F);
			float weight = (float) Math.pow(retention, dt * 60.0);
			blend(main, weight);
			active = true;
		} catch (RuntimeException e) {
			LOGGER.warn("Stabled: motion blur disabled after a rendering error", e);
			fail("Motion blur stopped after a rendering error. Toggle it off and on to retry.");
		}
	}

	// ----------------------------------------------------------------------------------------------------------
	// GPU work for this Minecraft version
	// ----------------------------------------------------------------------------------------------------------

//#if MC >= 26.2
	private static boolean pipelineReady() {
	//#if MC >= 26.3
		CompiledRenderPipeline compiled = RenderSystem.getCompiledPipelineNullable(PIPELINE);
		return compiled != null;
	//#else
		return RenderSystem.getDevice().precompilePipeline(PIPELINE).isValid();
	//#endif
	}

	private static void ensureTargets(int width, int height) {
		if (history == null || history.width != width || history.height != height) {
			destroyTargets();
	//#if MC >= 26.3
			history = new TextureTarget("Stabled motion blur history", width, height, GpuFormat.RGBA8_UNORM, null);
			scratch = new TextureTarget("Stabled motion blur output", width, height, GpuFormat.RGBA8_UNORM, null);
	//#else
			history = new TextureTarget("Stabled motion blur history", width, height, false, GpuFormat.RGBA8_UNORM);
			scratch = new TextureTarget("Stabled motion blur output", width, height, false, GpuFormat.RGBA8_UNORM);
	//#endif
			needsSeed = true;
		}
		if (config == null) {
			config = new MappableRingBuffer(() -> "Stabled motion blur config", GpuBuffer.USAGE_UNIFORM | GpuBuffer.USAGE_MAP_WRITE, 16);
		}
	}

	private static void copy(RenderTarget from, RenderTarget to) {
	//#if MC >= 26.3
		to.copyColorFrom(from);
	//#else
		RenderSystem.getDevice().createCommandEncoder()
			.copyTextureToTexture(from.getColorTexture(), to.getColorTexture(), 0, 0, 0, 0, 0, from.width, from.height);
	//#endif
	}

	/** main = mix(main, history, weight) in a shader pass into scratch, then scratch becomes the history. */
	private static void blend(RenderTarget main, float weight) {
		GpuBuffer ubo = config.currentBuffer();
		try (GpuBufferSlice.MappedView view = ubo.map(false, true)) {
			Std140Builder.intoBuffer(view.data()).putVec4(weight, 0F, 0F, 0F);
		}
	//#if MC >= 26.3
		try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder()
			.createRenderPass(() -> "Stabled motion blur", scratch.getColorTextureView(), Optional.empty())) {
			pass.setPipeline(RenderSystem.getCompiledPipelineNullable(PIPELINE));
			RenderSystem.bindDefaultUniforms(pass);
			pass.setUniform("InSampler", main.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
			pass.setUniform("PrevSampler", history.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
			pass.setUniform("MotionBlurConfig", ubo);
			pass.draw(3, 1, 0, 0);
		}
	//#else
		try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder()
			.createRenderPass(() -> "Stabled motion blur", scratch.getColorTextureView(), Optional.empty())) {
			pass.setPipeline(PIPELINE);
			RenderSystem.bindDefaultUniforms(pass);
			pass.bindTexture("InSampler", main.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
			pass.bindTexture("PrevSampler", history.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
			pass.setUniform("MotionBlurConfig", ubo);
			pass.draw(3, 1, 0, 0);
		}
	//#endif
		config.rotate();
		copy(scratch, main);
		TextureTarget swap = history;
		history = scratch;
		scratch = swap;
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
		if (config != null) {
			config.close();
			config = null;
		}
	}
//#elif MC >= 1.21.5
	private static boolean pipelineReady() {
		return true;
	}

	private static void ensureTargets(int width, int height) {
		if (history == null || history.width != width || history.height != height) {
			destroyTargets();
			history = new TextureTarget("Stabled motion blur history", width, height, false);
			needsSeed = true;
		}
	}

	private static void copy(RenderTarget from, RenderTarget to) {
		RenderSystem.getDevice().createCommandEncoder()
			.copyTextureToTexture(from.getColorTexture(), to.getColorTexture(), 0, 0, 0, 0, 0, from.width, from.height);
	}

	/** Blends the history over main with alpha = weight (GL constant blend colour), then main becomes the history. */
	private static void blend(RenderTarget main, float weight) {
		GL14.glBlendColor(0F, 0F, 0F, weight);
		try {
	//#if MC >= 1.21.11
			try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder()
				.createRenderPass(() -> "Stabled motion blur", main.getColorTextureView(), OptionalInt.empty())) {
				pass.setPipeline(PIPELINE);
				RenderSystem.bindDefaultUniforms(pass);
				pass.bindTexture("InSampler", history.getColorTextureView(), RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
				pass.draw(0, 3);
			}
	//#elif MC >= 1.21.9
			try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder()
				.createRenderPass(() -> "Stabled motion blur", main.getColorTextureView(), OptionalInt.empty())) {
				pass.setPipeline(PIPELINE);
				RenderSystem.bindDefaultUniforms(pass);
				pass.bindSampler("InSampler", history.getColorTextureView());
				pass.draw(0, 3);
			}
	//#else
			RenderSystem.AutoStorageIndexBuffer indices = RenderSystem.getSequentialBuffer(VertexFormat.Mode.QUADS);
			GpuBuffer indexBuffer = indices.getBuffer(6);
			GpuBuffer quad = RenderSystem.getQuadVertexBuffer();
		//#if MC >= 1.21.6
			try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder()
				.createRenderPass(() -> "Stabled motion blur", main.getColorTextureView(), OptionalInt.empty())) {
				pass.setPipeline(PIPELINE);
				RenderSystem.bindDefaultUniforms(pass);
				pass.setVertexBuffer(0, quad);
				pass.setIndexBuffer(indexBuffer, indices.type());
				pass.bindSampler("InSampler", history.getColorTextureView());
				pass.drawIndexed(0, 0, 6, 1);
			}
		//#else
			try (RenderPass pass = RenderSystem.getDevice().createCommandEncoder().createRenderPass(main.getColorTexture(), OptionalInt.empty())) {
				pass.setPipeline(PIPELINE);
				pass.setVertexBuffer(0, quad);
				pass.setIndexBuffer(indexBuffer, indices.type());
				pass.bindSampler("InSampler", history.getColorTexture());
				pass.drawIndexed(0, 6);
			}
		//#endif
	//#endif
		} finally {
			GL14.glBlendColor(0F, 0F, 0F, 0F);
		}
		copy(main, history);
	}

	private static void destroyTargets() {
		if (history != null) {
			history.destroyBuffers();
			history = null;
		}
	}
//#else
	private static boolean pipelineReady() {
		return true;
	}

	private static void ensureTargets(int width, int height) {
		if (history == null || history.width != width || history.height != height) {
			destroyTargets();
	//#if MC >= 1.21.2
			history = new TextureTarget(width, height, false);
	//#else
			history = new TextureTarget(width, height, false, Minecraft.ON_OSX);
	//#endif
			needsSeed = true;
		}
	}

	private static void copy(RenderTarget from, RenderTarget to) {
		GlStateManager._glBindFramebuffer(GL30.GL_READ_FRAMEBUFFER, from.frameBufferId);
		GlStateManager._glBindFramebuffer(GL30.GL_DRAW_FRAMEBUFFER, to.frameBufferId);
		GlStateManager._glBlitFrameBuffer(0, 0, from.width, from.height, 0, 0, to.width, to.height, GL11.GL_COLOR_BUFFER_BIT, GL11.GL_NEAREST);
		GlStateManager._glBindFramebuffer(GL30.GL_FRAMEBUFFER, 0);
		Minecraft.getInstance().getMainRenderTarget().bindWrite(true);
	}

	/** Blends the history over main with alpha = weight (GL constant blend colour), then main becomes the history. */
	private static void blend(RenderTarget main, float weight) {
		main.bindWrite(false);
		RenderSystem.enableBlend();
		RenderSystem.blendFunc(GlStateManager.SourceFactor.CONSTANT_ALPHA, GlStateManager.DestFactor.ONE_MINUS_CONSTANT_ALPHA);
		GL14.glBlendColor(0F, 0F, 0F, weight);
		try {
	//#if MC >= 1.21.2
			history.blitAndBlendToScreen(main.width, main.height);
	//#elif MC >= 1.21
			history.blitToScreen(main.width, main.height, false);
	//#else
			((RenderTargetInvoker) history).stabled$blitToScreen(main.width, main.height, false);
	//#endif
		} finally {
			GL14.glBlendColor(0F, 0F, 0F, 0F);
			RenderSystem.defaultBlendFunc();
			RenderSystem.disableBlend();
		}
		copy(main, history);
	}

	private static void destroyTargets() {
		if (history != null) {
			history.destroyBuffers();
			history = null;
		}
	}
//#endif

	private static void fail(String reason) {
		failure = reason;
		release();
	}

	/** Releases GPU resources. Safe to call every frame. Render thread only. */
	public static void release() {
		destroyTargets();
		needsSeed = true;
		lastLevel = null;
		active = false;
	}

	/** Clears a previous failure so the effect can be retried (used when the toggle is switched on again). */
	public static void resetFailure() {
		failure = null;
	}

	public static boolean isActive() {
		return active;
	}

	/** Non-null when the effect cannot run in the current configuration; shown next to the control. */
	public static String unavailableReason() {
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
