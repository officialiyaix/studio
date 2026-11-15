package stabled.client.mixin;

import com.mojang.realmsclient.gui.screens.RealmsNotificationsScreen;
import net.minecraft.client.gui.Font;
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
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
import stabled.client.ui.StabledStyle;
import stabled.client.ui.screen.StabledSettingsScreen;
import stabled.client.ui.screen.TitleLayout;
//#if MC >= 26.1
import net.minecraft.client.gui.GuiGraphicsExtractor;
//#else
import net.minecraft.client.gui.GuiGraphics;
//#endif
//#if MC < 1.20.5
import net.minecraft.client.renderer.PanoramaRenderer;
import net.minecraft.resources.ResourceLocation;
//#endif

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
		for (AbstractWidget w : Platform.widgets(this)) {
			if (w.getMessage().getContents() instanceof TranslatableContents t && "menu.online".equals(t.getKey())) {
				realms = w;
			}
		}
		if (realms != null) {
			int y = realms.getY() + 24;
			for (AbstractWidget w : Platform.widgets(this)) {
				if (w.getY() >= y && w.getY() < this.height - 12) {
					w.setY(w.getY() + 24);
				}
			}
			this.addRenderableWidget(Button.builder(Component.translatable("stabled.menu.settings"),
				b -> Platform.setScreen(new StabledSettingsScreen(this))).bounds(realms.getX(), y, realms.getWidth(), 20).build());
		}
	}

//#if MC >= 26.1
	@Redirect(method = "extractRenderState", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/screens/TitleScreen;extractPanorama(Lnet/minecraft/client/gui/GuiGraphicsExtractor;F)V"))
	private void stabled$background(TitleScreen self, GuiGraphicsExtractor graphics, float a) {
		if (StabledStyle.menusStyled()) {
			TitleLayout.background(self, new Gfx(graphics), a);
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
//#else
	//#if MC >= 1.20.5
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/screens/TitleScreen;renderPanorama(Lnet/minecraft/client/gui/GuiGraphics;F)V"))
	private void stabled$background(TitleScreen self, GuiGraphics graphics, float a) {
		if (StabledStyle.menusStyled()) {
			TitleLayout.background(self, new Gfx(graphics), a);
		} else {
			this.renderPanorama(graphics, a);
		}
	}
	//#else
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/renderer/PanoramaRenderer;render(FF)V"))
	private void stabled$panorama(PanoramaRenderer panorama, float delta, float alpha) {
		if (!StabledStyle.menusStyled()) {
			panorama.render(delta, alpha);
		}
	}

	/** Up to 1.20.4 the panorama is followed by a full-screen overlay blit; the Stabled backdrop replaces both. */
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/GuiGraphics;blit(Lnet/minecraft/resources/ResourceLocation;IIIIFFIIII)V", ordinal = 0))
	private void stabled$background(GuiGraphics graphics, ResourceLocation texture, int x, int y, int w, int h, float u, float v, int srcW, int srcH, int texW, int texH) {
		if (StabledStyle.menusStyled()) {
			graphics.setColor(1F, 1F, 1F, 1F);
			TitleLayout.background(this, new Gfx(graphics), 0F);
		} else {
			graphics.blit(texture, x, y, w, h, u, v, srcW, srcH, texW, texH);
		}
	}
	//#endif

	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/LogoRenderer;renderLogo(Lnet/minecraft/client/gui/GuiGraphics;IF)V"))
	private void stabled$logo(LogoRenderer logo, GuiGraphics graphics, int width, float alpha) {
		if (!StabledStyle.menusStyled()) {
			logo.renderLogo(graphics, width, alpha);
		}
	}

	//#if MC >= 1.21.6
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/SplashRenderer;render(Lnet/minecraft/client/gui/GuiGraphics;ILnet/minecraft/client/gui/Font;F)V"))
	private void stabled$splash(SplashRenderer splash, GuiGraphics graphics, int width, Font font, float alpha) {
		if (!StabledStyle.menusStyled()) {
			splash.render(graphics, width, font, alpha);
		}
	}
	//#else
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lnet/minecraft/client/gui/components/SplashRenderer;render(Lnet/minecraft/client/gui/GuiGraphics;ILnet/minecraft/client/gui/Font;I)V"))
	private void stabled$splash(SplashRenderer splash, GuiGraphics graphics, int width, Font font, int color) {
		if (!StabledStyle.menusStyled()) {
			splash.render(graphics, width, font, color);
		}
	}
	//#endif

	/** The Realms invite badge is positioned for the vanilla layout, so it is hidden in the Stabled layout. */
	@Redirect(method = "render", at = @At(value = "INVOKE", target = "Lcom/mojang/realmsclient/gui/screens/RealmsNotificationsScreen;render(Lnet/minecraft/client/gui/GuiGraphics;IIF)V"))
	private void stabled$realmsBadge(RealmsNotificationsScreen notifications, GuiGraphics graphics, int mouseX, int mouseY, float a) {
		if (!StabledStyle.menusStyled()) {
			notifications.render(graphics, mouseX, mouseY, a);
		}
	}
//#endif
}
