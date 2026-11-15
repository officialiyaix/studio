package stabled.client.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.theme.Theme;
import stabled.client.ui.screen.StabledScreen;

/**
 * Decides where Stabled's design language is applied to vanilla widgets. Only the supported menus are styled
 * (title screen, pause menu and Stabled's own screens); every other vanilla and modded screen is left untouched.
 */
public final class StabledStyle {
	private StabledStyle() {
	}

	public static boolean menusStyled() {
		return StabledClient.isReady() && StabledClient.settings().styledMenus;
	}

	public static boolean appliesTo(Screen screen) {
		if (screen instanceof StabledScreen) {
			return true;
		}
		return menusStyled() && (screen instanceof TitleScreen || screen instanceof PauseScreen);
	}

	public static boolean stylingCurrentScreen() {
		Minecraft mc = Minecraft.getInstance();
		Screen screen = mc.gui != null ? mc.gui.screen() : null;
		return screen != null && appliesTo(screen);
	}

	/** Light themes use accent-filled buttons so vanilla's white icons and labels stay readable. */
	public static void drawVanillaButton(GuiGraphicsExtractor g, AbstractButton button) {
		Theme t = Ui.theme();
		boolean hot = button.isHoveredOrFocused() && button.active;
		int x = button.getX(), y = button.getY(), w = button.getWidth(), h = button.getHeight();
		int base = t.light() ? Ui.mix(t.accent(), t.text(), 0.15F) : t.surfaceAlt();
		int fill = hot ? (t.light() ? Ui.mix(base, 0xFFFFFFFF, 0.15F) : t.surfaceHover()) : base;
		float opacity = Math.max(0.4F, Ui.appearance().surfaceOpacity());
		if (!button.active) {
			fill = Ui.alpha(fill, 0.5F);
		}
		Ui.roundRect(g, x, y, w, h, 3, Ui.alpha(fill, (fill >>> 24) / 255F * opacity));
		int border = hot ? t.accent() : (t.light() ? Ui.mix(base, 0xFF000000, 0.2F) : t.border());
		Ui.roundOutline(g, x, y, w, h, 3, Ui.outline(border));
		if (hot && !t.light()) {
			g.fill(x + 3, y + h - 2, x + w - 3, y + h - 1, Ui.alpha(t.accent(), 0.9F));
		}
	}

	public static Component recolorLabel(AbstractButton button, Component message) {
		Theme t = Ui.theme();
		int color = t.light() ? 0xFFFFFF : (button.active ? t.text() : t.textMuted()) & 0xFFFFFF;
		if (t.light() && !button.active) {
			color = 0xE6E6EE;
		}
		int finalColor = color;
		return message.copy().withStyle(s -> s.getColor() == null ? s.withColor(finalColor) : s);
	}
}
