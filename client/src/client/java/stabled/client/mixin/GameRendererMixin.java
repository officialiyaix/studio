package stabled.client.mixin;

import com.mojang.blaze3d.pipeline.RenderTarget;
import net.minecraft.client.renderer.GameRenderer;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.render.MotionBlurRenderer;

/**
 * Runs motion blur on the main render target right after the level and vanilla post effects, and before
 * {@code guiRenderer.render()}, so only the 3D world is blurred.
 */
@Mixin(GameRenderer.class)
public abstract class GameRendererMixin {
	@Shadow
	@Final
	private RenderTarget mainRenderTarget;

	@Inject(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/renderer/GameRenderer;applyPostEffects()V", shift = At.Shift.AFTER))
	private void stabled$motionBlur(CallbackInfo ci) {
		MotionBlurRenderer.afterLevel(this.mainRenderTarget);
	}

	@Inject(method = "close", at = @At("HEAD"))
	private void stabled$releaseMotionBlur(CallbackInfo ci) {
		MotionBlurRenderer.release();
	}
}
