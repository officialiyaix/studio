package stabled.client.mixin;

import com.mojang.realmsclient.gui.screens.RealmsNotificationsScreen;
import net.fabricmc.fabric.api.client.screen.v1.Screens;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.LogoRenderer;
import net.minecraft.client.gui.components.SplashRenderer;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.contents.TranslatableContents;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.ui.StabledStyle;
import stabled.client.ui.screen.StabledSettingsScreen;
import stabled.client.ui.screen.TitleLayout;

/** Stabled main menu: branded layout over the vanilla title screen, which keeps all of its behaviour. */
@Mixin(TitleScreen.class)
public abstract class TitleScreenMixin extends Screen {
	protected TitleScreenMixin(Component title) {
		super(title);
	}

	@Inject(method = "init", at = @At("TAIL"))
	private void stabled$layout(CallbackInfo ci) {
		if (StabledStyle.menusStyled()) {
			TitleLayout.apply((TitleScreen) (Object) this, this::addRenderableWidget, this.width, this.height);
			return;
		}
		// Unstyled: keep the vanilla layout and insert a Stabled Settings button below Realms.
		AbstractWidget realms = null;
		for (AbstractWidget w : Screens.getWidgets(this)) {
			if (w.getMessage().getContents() instanceof TranslatableContents t && "menu.online".equals(t.getKey())) {
				realms = w;
			}
		}
		if (realms != null) {
			int y = realms.getY() + 24;
			for (AbstractWidget w : Screens.getWidgets(this)) {
				if (w.getY() >= y && w.getY() < this.height - 12) {
					w.setY(w.getY() + 24);
				}
			}
			this.addRenderableWidget(Button.builder(Component.translatable("stabled.menu.settings"),
				b -> this.minecraft.gui.setScreen(new StabledSettingsScreen(this))).bounds(realms.getX(), y, realms.getWidth(), 20).build());
		}
	}

	@Redirect(method = "extractRenderState", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/screens/TitleScreen;extractPanorama(Lnet/minecraft/client/gui/GuiGraphicsExtractor;F)V"))
	private void stabled$background(TitleScreen self, GuiGraphicsExtractor graphics, float a) {
		if (StabledStyle.menusStyled()) {
			TitleLayout.background(self, graphics, a);
		} else {
			this.extractPanorama(graphics, a);
		}
	}

	@Redirect(method = "extractRenderState", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/LogoRenderer;extractRenderState(Lnet/minecraft/client/gui/GuiGraphicsExtractor;IF)V"))
	private void stabled$logo(LogoRenderer logo, GuiGraphicsExtractor graphics, int width, float alpha) {
		if (!StabledStyle.menusStyled()) {
			logo.extractRenderState(graphics, width, alpha);
		}
	}

	@Redirect(method = "extractRenderState", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/SplashRenderer;extractRenderState(Lnet/minecraft/client/gui/GuiGraphicsExtractor;ILnet/minecraft/client/gui/Font;F)V"))
	private void stabled$splash(SplashRenderer splash, GuiGraphicsExtractor graphics, int width, Font font, float alpha) {
		if (!StabledStyle.menusStyled()) {
			splash.extractRenderState(graphics, width, font, alpha);
		}
	}

	/** The Realms invite badge is positioned for the vanilla layout, so it is hidden in the Stabled layout. */
	@Redirect(method = "extractRenderState", at = @At(value = "INVOKE", target = "Lcom/mojang/realmsclient/gui/screens/RealmsNotificationsScreen;extractRenderState(Lnet/minecraft/client/gui/GuiGraphicsExtractor;IIF)V"))
	private void stabled$realmsBadge(RealmsNotificationsScreen notifications, GuiGraphicsExtractor graphics, int mouseX, int mouseY, float a) {
		if (!StabledStyle.menusStyled()) {
			notifications.extractRenderState(graphics, mouseX, mouseY, a);
		}
	}
}
