package stabled.client.mixin;

import net.minecraft.client.renderer.GameRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.platform.MotionBlur;
//#if MC >= 26.3
import com.mojang.blaze3d.pipeline.RenderTarget;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Shadow;
//#else
import net.minecraft.client.Minecraft;
//#endif

/**
 * Runs motion blur on the main render target right after the level and vanilla post effects are drawn and before
 * the GUI is, so only the 3D world is blurred.
 */
@Mixin(GameRenderer.class)
public abstract class GameRendererMixin {
//#if MC >= 26.3
	@Shadow
	@Final
	private RenderTarget mainRenderTarget;

	@Inject(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/renderer/GameRenderer;applyPostEffects()V", shift = At.Shift.AFTER))
	private void stabled$motionBlur(CallbackInfo ci) {
		MotionBlur.afterLevel(this.mainRenderTarget);
	}
//#elif MC >= 1.21.5
	// The depth clear that follows the level and its post effects, before the GUI is drawn.
	@Inject(method = "render", at = @At(value = "INVOKE", target = "Lcom/mojang/blaze3d/systems/CommandEncoder;clearDepthTexture(Lcom/mojang/blaze3d/textures/GpuTexture;D)V", ordinal = 0))
	private void stabled$motionBlur(CallbackInfo ci) {
	//#if MC >= 26.2
		MotionBlur.afterLevel(Minecraft.getInstance().gameRenderer.mainRenderTarget());
	//#else
		MotionBlur.afterLevel(Minecraft.getInstance().getMainRenderTarget());
	//#endif
	}
//#else
	// Vanilla rebinds the main target after the level and its post effects, before the GUI is drawn.
	@Inject(method = "render", at = @At(value = "INVOKE", target = "Lcom/mojang/blaze3d/pipeline/RenderTarget;bindWrite(Z)V", ordinal = 0))
	private void stabled$motionBlur(CallbackInfo ci) {
		MotionBlur.afterLevel(Minecraft.getInstance().getMainRenderTarget());
	}
//#endif

	@Inject(method = "close", at = @At("HEAD"))
	private void stabled$releaseMotionBlur(CallbackInfo ci) {
		MotionBlur.release();
	}
}
