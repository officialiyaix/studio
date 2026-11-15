package stabled.client.ui;

import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
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
		Screen screen = Platform.screen();
		return screen != null && appliesTo(screen);
	}

	/** Light themes use accent-filled buttons so vanilla's white icons and labels stay readable. */
	public static void drawVanillaButton(Gfx g, AbstractButton button) {
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

	/** RGB label colour for a vanilla button on a styled menu (labels with their own colour keep it). */
	public static int labelColor(AbstractButton button) {
		Theme t = Ui.theme();
		if (t.light()) {
			return button.active ? 0xFFFFFF : 0xE6E6EE;
		}
		return (button.active ? t.text() : t.textMuted()) & 0xFFFFFF;
	}

	public static Component recolorLabel(AbstractButton button, Component message) {
		int color = labelColor(button);
		return message.copy().withStyle(s -> s.getColor() == null ? s.withColor(color) : s);
	}

	/** Stabled pause menu: a translucent wash over the world and a themed panel behind the vanilla buttons. */
	public static void drawPausePanel(Gfx g, Screen screen) {
		Ui.worldScrim(g, screen.width, screen.height);
		int minX = Integer.MAX_VALUE, minY = Integer.MAX_VALUE, maxX = Integer.MIN_VALUE, maxY = Integer.MIN_VALUE;
		for (AbstractWidget w : Platform.widgets(screen)) {
			if (w instanceof Button) {
				minX = Math.min(minX, w.getX());
				minY = Math.min(minY, w.getY());
				maxX = Math.max(maxX, w.getX() + w.getWidth());
				maxY = Math.max(maxY, w.getY() + w.getHeight());
			}
		}
		if (minX == Integer.MAX_VALUE) {
			return;
		}
		Theme t = Ui.theme();
		int pad = 10;
		int x = minX - pad, y = minY - pad - 22, w = maxX - minX + pad * 2, h = maxY - minY + pad * 2 + 22;
		Ui.roundRect(g, x, y, w, h, 5, Ui.surface(t.surface()));
		Ui.roundOutline(g, x, y, w, h, 5, Ui.outline(t.border()));
		g.fill(x + 1, y + 1, x + w - 1, y + 2, Ui.alpha(t.accent(), 0.7F));
		Ui.mark(g, x + 8, y + 7, 12);
	}
}
