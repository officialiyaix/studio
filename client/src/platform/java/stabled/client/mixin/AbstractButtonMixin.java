package stabled.client.mixin;

import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.platform.Gfx;
import stabled.client.ui.StabledStyle;
//#if MC >= 26.1
import net.minecraft.client.gui.GuiGraphicsExtractor;
//#else
import net.minecraft.client.gui.GuiGraphics;
//#endif
//#if MC >= 1.21.11
import org.spongepowered.asm.mixin.injection.ModifyArg;
//#else
import net.minecraft.client.Minecraft;
import net.minecraft.util.Mth;
//#endif

/** Themes vanilla buttons, but only on the menus Stabled supports (see {@link StabledStyle}). */
@Mixin(AbstractButton.class)
public abstract class AbstractButtonMixin extends AbstractWidget {
	protected AbstractButtonMixin(int x, int y, int width, int height, Component message) {
		super(x, y, width, height, message);
	}

//#if MC >= 26.1
	@Inject(method = "extractDefaultSprite", at = @At("HEAD"), cancellable = true)
	private void stabled$themedSprite(GuiGraphicsExtractor graphics, CallbackInfo ci) {
		if (StabledStyle.stylingCurrentScreen()) {
			StabledStyle.drawVanillaButton(new Gfx(graphics), (AbstractButton) (Object) this);
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
//#elif MC >= 1.21.11
	@Inject(method = "renderDefaultSprite", at = @At("HEAD"), cancellable = true)
	private void stabled$themedSprite(GuiGraphics graphics, CallbackInfo ci) {
		if (StabledStyle.stylingCurrentScreen()) {
			StabledStyle.drawVanillaButton(new Gfx(graphics), (AbstractButton) (Object) this);
			ci.cancel();
		}
	}

	@ModifyArg(
		method = "renderDefaultLabel",
		at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/AbstractButton;renderScrollingStringOverContents(Lnet/minecraft/client/gui/ActiveTextCollector;Lnet/minecraft/network/chat/Component;I)V"),
		index = 1
	)
	private Component stabled$themedLabel(Component message) {
		return StabledStyle.stylingCurrentScreen() ? StabledStyle.recolorLabel((AbstractButton) (Object) this, message) : message;
	}
//#else
	/** Replaces the vanilla sprite and label colour; buttons that draw themselves (icons, checkboxes) are untouched. */
	@Inject(method = "renderWidget", at = @At("HEAD"), cancellable = true)
	private void stabled$themed(GuiGraphics graphics, int mouseX, int mouseY, float partialTick, CallbackInfo ci) {
		if (StabledStyle.stylingCurrentScreen()) {
			AbstractButton self = (AbstractButton) (Object) this;
			StabledStyle.drawVanillaButton(new Gfx(graphics), self);
			self.renderString(graphics, Minecraft.getInstance().font, StabledStyle.labelColor(self) | Mth.ceil(this.alpha * 255.0F) << 24);
			ci.cancel();
		}
	}
//#endif
}
