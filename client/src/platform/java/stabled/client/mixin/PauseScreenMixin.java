package stabled.client.mixin;

import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
import stabled.client.ui.StabledStyle;
import stabled.client.ui.Ui;
import stabled.client.ui.screen.StabledSettingsScreen;
//#if MC >= 26.1
import net.minecraft.client.gui.GuiGraphicsExtractor;
//#else
import net.minecraft.client.gui.GuiGraphics;
//#endif
//#if MC < 1.20.2
import org.spongepowered.asm.mixin.injection.Redirect;
//#endif

/** Adds Stabled Settings to the pause menu and gives it the Stabled panel treatment. */
@Mixin(PauseScreen.class)
public abstract class PauseScreenMixin extends Screen {
	@Shadow
	@Final
	private boolean showPauseMenu;

	@Shadow
	private Button disconnectButton;

//#if MC >= 26.2
	@Shadow
	private boolean isTopmostScreen() {
		throw new AssertionError();
	}

//#endif
	protected PauseScreenMixin(Component title) {
		super(title);
	}

	@Inject(method = "init", at = @At("TAIL"))
	private void stabled$addSettings(CallbackInfo ci) {
		if (!this.showPauseMenu || this.disconnectButton == null) {
			return;
		}
		int y = this.disconnectButton.getY();
		for (AbstractWidget w : Platform.widgets(this)) {
			if (w.getY() >= y) {
				w.setY(w.getY() + 24);
			}
		}
		this.addRenderableWidget(Button.builder(Component.translatable("stabled.menu.settings"),
			b -> Platform.setScreen(new StabledSettingsScreen(this)))
			.bounds(this.disconnectButton.getX(), y, this.disconnectButton.getWidth(), 20).build());
	}

//#if MC >= 1.20.2
	/**
	 * Styled menus replace the vanilla dark wash with the Stabled panel. Simple surfaces drop the menu blur; Frosted
	 * Glass keeps vanilla's blur (and the player's blur setting) on versions that have one (1.20.5+).
	 */
	//#if MC >= 26.1
	@Inject(method = "extractBackground", at = @At("HEAD"), cancellable = true)
	private void stabled$background(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float partialTick, CallbackInfo ci) {
	//#else
	@Inject(method = "renderBackground", at = @At("HEAD"), cancellable = true)
	private void stabled$background(GuiGraphics graphics, int mouseX, int mouseY, float partialTick, CallbackInfo ci) {
	//#endif
		if (!StabledStyle.menusStyled() || !this.showPauseMenu) {
			return;
		}
		if (Ui.appearance().isGlass()) {
	//#if MC >= 26.2
			if (this.isTopmostScreen()) {
				this.extractBlurredBackground(graphics);
			}
	//#elif MC >= 26.1
			this.extractBlurredBackground(graphics);
	//#elif MC >= 1.21.6
			this.renderBlurredBackground(graphics);
	//#elif MC >= 1.21.2
			this.renderBlurredBackground();
	//#elif MC >= 1.20.5
			this.renderBlurredBackground(partialTick);
	//#endif
		}
		StabledStyle.drawPausePanel(new Gfx(graphics), this);
	//#if MC >= 26.2
		this.minecraft.gui.hud.extractDeferredSubtitles();
	//#elif MC >= 26.1
		this.minecraft.gui.extractDeferredSubtitles();
	//#elif MC >= 1.21.9
		this.minecraft.gui.renderDeferredSubtitles();
	//#endif
		ci.cancel();
	}
//#else
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/screens/PauseScreen;renderBackground(Lnet/minecraft/client/gui/GuiGraphics;)V"))
	private void stabled$background(PauseScreen self, GuiGraphics graphics) {
		if (StabledStyle.menusStyled()) {
			StabledStyle.drawPausePanel(new Gfx(graphics), this);
		} else {
			this.renderBackground(graphics);
		}
	}
//#endif
}
