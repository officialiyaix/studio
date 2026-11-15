package stabled.client.mixin;

import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyArg;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.ui.StabledStyle;

/** Themes vanilla buttons, but only on the menus Stabled supports (see {@link StabledStyle}). */
@Mixin(AbstractButton.class)
public abstract class AbstractButtonMixin {
	@Inject(method = "extractDefaultSprite", at = @At("HEAD"), cancellable = true)
	private void stabled$themedSprite(GuiGraphicsExtractor graphics, CallbackInfo ci) {
		if (StabledStyle.stylingCurrentScreen()) {
			StabledStyle.drawVanillaButton(graphics, (AbstractButton) (Object) this);
			ci.cancel();
		}
	}

	@ModifyArg(
		method = "extractDefaultLabel",
		at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/AbstractButton;extractScrollingStringOverContents(Lnet/minecraft/client/gui/ActiveTextCollector;Lnet/minecraft/network/chat/Component;I)V"),
		index = 1
	)
	private Component stabled$themedLabel(Component message) {
		return StabledStyle.stylingCurrentScreen() ? StabledStyle.recolorLabel((AbstractButton) (Object) this, message) : message;
	}
}
