package stabled.client.mixin;

import net.fabricmc.fabric.api.client.screen.v1.Screens;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.theme.Theme;
import stabled.client.ui.StabledStyle;
import stabled.client.ui.Ui;
import stabled.client.ui.screen.StabledSettingsScreen;

/** Adds Stabled Settings to the pause menu and gives it the Stabled panel treatment. */
@Mixin(PauseScreen.class)
public abstract class PauseScreenMixin extends Screen {
	@Shadow
	@Final
	private boolean showPauseMenu;

	@Shadow
	private @Nullable Button disconnectButton;

	protected PauseScreenMixin(Component title) {
		super(title);
	}

	@Inject(method = "init", at = @At("TAIL"))
	private void stabled$addSettings(CallbackInfo ci) {
		if (!this.showPauseMenu || this.disconnectButton == null) {
			return;
		}
		int y = this.disconnectButton.getY();
		for (AbstractWidget w : Screens.getWidgets(this)) {
			if (w.getY() >= y) {
				w.setY(w.getY() + 24);
			}
		}
		this.addRenderableWidget(Button.builder(Component.translatable("stabled.menu.settings"),
			b -> this.minecraft.gui.setScreen(new StabledSettingsScreen(this)))
			.bounds(this.disconnectButton.getX(), y, this.disconnectButton.getWidth(), 20).build());
	}

	/** Simple surfaces remove the menu blur; Frosted Glass keeps vanilla's blur (and the player's blur setting). */
	@Redirect(method = "extractBackground", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/screens/PauseScreen;extractBlurredBackground(Lnet/minecraft/client/gui/GuiGraphicsExtractor;)V"))
	private void stabled$blur(PauseScreen self, GuiGraphicsExtractor graphics) {
		if (!StabledStyle.menusStyled() || Ui.appearance().isGlass()) {
			this.extractBlurredBackground(graphics);
		}
	}

	@Redirect(method = "extractBackground", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/screens/PauseScreen;extractMenuBackground(Lnet/minecraft/client/gui/GuiGraphicsExtractor;)V"))
	private void stabled$panel(PauseScreen self, GuiGraphicsExtractor graphics) {
		if (!StabledStyle.menusStyled()) {
			this.extractMenuBackground(graphics);
			return;
		}
		Ui.worldScrim(graphics, this.width, this.height);
		int minX = Integer.MAX_VALUE, minY = Integer.MAX_VALUE, maxX = Integer.MIN_VALUE, maxY = Integer.MIN_VALUE;
		for (AbstractWidget w : Screens.getWidgets(this)) {
			if (w instanceof Button) {
				minX = Math.min(minX, w.getX());
				minY = Math.min(minY, w.getY());
				maxX = Math.max(maxX, w.getRight());
				maxY = Math.max(maxY, w.getBottom());
			}
		}
		if (minX == Integer.MAX_VALUE) {
			return;
		}
		Theme t = Ui.theme();
		int pad = 10;
		int x = minX - pad, y = minY - pad - 22, w = maxX - minX + pad * 2, h = maxY - minY + pad * 2 + 22;
		Ui.roundRect(graphics, x, y, w, h, 5, Ui.surface(t.surface()));
		Ui.roundOutline(graphics, x, y, w, h, 5, Ui.outline(t.border()));
		graphics.fill(x + 1, y + 1, x + w - 1, y + 2, Ui.alpha(t.accent(), 0.7F));
		Ui.mark(graphics, x + 8, y + 7, 12);
	}
}
