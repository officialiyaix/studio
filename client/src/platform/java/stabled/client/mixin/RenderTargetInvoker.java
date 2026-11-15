//#if MC < 1.21
package stabled.client.mixin;

import com.mojang.blaze3d.pipeline.RenderTarget;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Invoker;

/**
 * Before 1.21, {@code RenderTarget.blitToScreen} queues the draw for later instead of running it, so motion blur
 * calls the immediate private implementation directly.
 */
@Mixin(RenderTarget.class)
public interface RenderTargetInvoker {
	@Invoker("_blitToScreen")
	void stabled$blitToScreen(int width, int height, boolean disableBlend);
}
//#endif
