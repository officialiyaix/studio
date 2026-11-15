package stabled.client.ui;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.util.FormattedCharSequence;
import stabled.client.config.Appearance;
import stabled.client.theme.Theme;
import stabled.client.theme.ThemeManager;

/** Drawing helpers for Stabled's in-game design language (rounded panels, theme colours, glass surfaces). */
public final class Ui {
	public static final Identifier LOGO = Identifier.fromNamespaceAndPath("stabled", "textures/gui/logo.png");
	public static final Identifier LOGO_DARK_TEXT = Identifier.fromNamespaceAndPath("stabled", "textures/gui/logo_dark.png");
	public static final Identifier MARK = Identifier.fromNamespaceAndPath("stabled", "textures/gui/mark.png");
	public static final int LOGO_W = 512;
	public static final int LOGO_H = 128;
	public static final int MARK_SIZE = 128;

	private Ui() {
	}

	public static Theme theme() {
		return ThemeManager.current();
	}

	public static Appearance appearance() {
		return ThemeManager.appearance();
	}

	public static int alpha(int argb, float alpha) {
		int a = Math.round(Math.max(0F, Math.min(1F, alpha)) * ((argb >>> 24) & 0xFF));
		return (a << 24) | (argb & 0xFFFFFF);
	}

	public static int mix(int a, int b, float t) {
		t = Math.max(0F, Math.min(1F, t));
		int aa = (a >>> 24) & 0xFF, ar = (a >> 16) & 0xFF, ag = (a >> 8) & 0xFF, ab = a & 0xFF;
		int ba = (b >>> 24) & 0xFF, br = (b >> 16) & 0xFF, bg = (b >> 8) & 0xFF, bb = b & 0xFF;
		return (Math.round(aa + (ba - aa) * t) << 24) | (Math.round(ar + (br - ar) * t) << 16)
			| (Math.round(ag + (bg - ag) * t) << 8) | Math.round(ab + (bb - ab) * t);
	}

	/** Opaque text colour (text with zero alpha is not drawn, so always force it). */
	public static int opaque(int rgb) {
		return 0xFF000000 | rgb;
	}

	/** Surface colour honouring Simple (opaque) or Frosted Glass (translucent) style. */
	public static int surface(int color) {
		return alpha(color, appearance().surfaceOpacity());
	}

	/** Borders stay a little more visible than fills on glass so edges remain readable. */
	public static int outline(int color) {
		Appearance a = appearance();
		return a.isGlass() ? alpha(color, Math.min(1F, a.surfaceOpacity() + 0.35F)) : color;
	}

	public static void roundRect(GuiGraphicsExtractor g, int x, int y, int w, int h, int r, int color) {
		if (w <= 0 || h <= 0 || ((color >>> 24) == 0)) {
			return;
		}
		r = Math.max(0, Math.min(r, Math.min(w, h) / 2));
		if (r == 0) {
			g.fill(x, y, x + w, y + h, color);
			return;
		}
		g.fill(x, y + r, x + w, y + h - r, color);
		for (int i = 0; i < r; i++) {
			int inset = inset(r, i);
			g.fill(x + inset, y + i, x + w - inset, y + i + 1, color);
			g.fill(x + inset, y + h - i - 1, x + w - inset, y + h - i, color);
		}
	}

	public static void roundOutline(GuiGraphicsExtractor g, int x, int y, int w, int h, int r, int color) {
		if (w <= 1 || h <= 1 || ((color >>> 24) == 0)) {
			return;
		}
		r = Math.max(0, Math.min(r, Math.min(w, h) / 2));
		g.fill(x + r, y, x + w - r, y + 1, color);
		g.fill(x + r, y + h - 1, x + w - r, y + h, color);
		g.fill(x, y + r, x + 1, y + h - r, color);
		g.fill(x + w - 1, y + r, x + w, y + h - r, color);
		for (int i = 1; i < r; i++) {
			int inset = inset(r, i);
			int prev = inset(r, i - 1);
			// Left/right corner columns for row i, spanning the horizontal step from the previous row.
			int from = Math.min(inset, prev);
			int to = Math.max(inset, prev);
			g.fill(x + from, y + i, x + to + 1, y + i + 1, color);
			g.fill(x + w - to - 1, y + i, x + w - from, y + i + 1, color);
			g.fill(x + from, y + h - i - 1, x + to + 1, y + h - i, color);
			g.fill(x + w - to - 1, y + h - i - 1, x + w - from, y + h - i, color);
		}
	}

	private static int inset(int r, int row) {
		double dy = r - row - 0.5;
		return (int) Math.round(r - Math.sqrt(Math.max(0, r * r - dy * dy)));
	}

	/** A themed panel: surface fill plus a subtle outline. */
	public static void panel(GuiGraphicsExtractor g, int x, int y, int w, int h, int radius) {
		Theme t = theme();
		roundRect(g, x, y, w, h, radius, surface(t.surface()));
		roundOutline(g, x, y, w, h, radius, outline(t.border()));
	}

	public static void panelAlt(GuiGraphicsExtractor g, int x, int y, int w, int h, int radius) {
		Theme t = theme();
		roundRect(g, x, y, w, h, radius, surface(t.surfaceAlt()));
		roundOutline(g, x, y, w, h, radius, outline(t.border()));
	}

	/**
	 * Full-screen Stabled backdrop for menus outside a world: the custom background image (cover-fit around the
	 * chosen focal point, with a readability dim) or the theme's restrained gradient.
	 */
	public static void backdrop(GuiGraphicsExtractor g, int width, int height) {
		Theme t = theme();
		if (ThemeManager.isCustomSelected()) {
			Identifier tex = ThemeManager.customBackgroundTexture();
			if (tex != null) {
				coverBlit(g, tex, 0, 0, width, height, ThemeManager.backgroundWidth(), ThemeManager.backgroundHeight());
				g.fill(0, 0, width, height, alpha(0xFF000000, (float) ThemeManager.customDim()));
				return;
			}
		}
		g.fillGradient(0, 0, width, height, t.bg(), t.bgAlt());
		// Soft accent glow along the top edge keeps the gradient restrained but recognisable.
		g.fillGradient(0, 0, width, Math.max(1, height / 3), alpha(t.accent(), 0.10F), alpha(t.accent(), 0F));
	}

	/** Backdrop for menus outside a world: the custom image, or the Minecraft panorama tinted with the theme. */
	public static void menuBackdrop(GuiGraphicsExtractor g, int width, int height) {
		if (ThemeManager.isCustomSelected() && ThemeManager.customBackgroundTexture() != null) {
			backdrop(g, width, height);
			return;
		}
		Theme t = theme();
		Minecraft.getInstance().gameRenderer.panorama().extractRenderState(g, width, height);
		g.fillGradient(0, 0, width, height, alpha(t.bg(), 0.55F), alpha(t.bgAlt(), 0.70F));
	}

	/** Translucent wash drawn over the live world or panorama behind Stabled menus. */
	public static void worldScrim(GuiGraphicsExtractor g, int width, int height) {
		Theme t = theme();
		float strength = appearance().isGlass() ? 0.35F : 0.72F;
		g.fillGradient(0, 0, width, height, alpha(t.bg(), strength), alpha(t.bgAlt(), strength + 0.08F));
	}

	public static void coverBlit(GuiGraphicsExtractor g, Identifier tex, int x, int y, int w, int h, int texW, int texH) {
		if (texW <= 0 || texH <= 0) {
			return;
		}
		Appearance.CustomBackground bg = appearance().customBackground;
		double fx = bg != null ? bg.focusX : 0.5;
		double fy = bg != null ? bg.focusY : 0.5;
		double scale = Math.max(w / (double) texW, h / (double) texH);
		int srcW = Math.max(1, (int) Math.round(w / scale));
		int srcH = Math.max(1, (int) Math.round(h / scale));
		float u = (float) ((texW - srcW) * fx);
		float v = (float) ((texH - srcH) * fy);
		g.blit(RenderPipelines.GUI_TEXTURED, tex, x, y, u, v, w, h, srcW, srcH, texW, texH);
	}

	public static void text(GuiGraphicsExtractor g, Component text, int x, int y, int color) {
		g.text(font(), text, x, y, opaque(color), false);
	}

	public static void textShadow(GuiGraphicsExtractor g, Component text, int x, int y, int color) {
		g.text(font(), text, x, y, opaque(color), true);
	}

	public static void centered(GuiGraphicsExtractor g, Component text, int cx, int y, int color) {
		Font f = font();
		g.text(f, text, cx - f.width(text) / 2, y, opaque(color), false);
	}

	/** Draws text clipped with an ellipsis so long labels never overflow their box. */
	public static void textClipped(GuiGraphicsExtractor g, Component text, int x, int y, int maxWidth, int color) {
		Font f = font();
		if (f.width(text) <= maxWidth) {
			g.text(f, text, x, y, opaque(color), false);
			return;
		}
		String ellipsis = "…";
		String plain = text.getString();
		String cut = f.plainSubstrByWidth(plain, Math.max(0, maxWidth - f.width(ellipsis)));
		g.text(f, cut + ellipsis, x, y, opaque(color), false);
	}

	public static int wrapped(GuiGraphicsExtractor g, Component text, int x, int y, int width, int color) {
		Font f = font();
		int lineY = y;
		for (FormattedCharSequence line : f.split(text, Math.max(10, width))) {
			g.text(f, line, x, lineY, opaque(color), false);
			lineY += f.lineHeight + 1;
		}
		return lineY - y;
	}

	public static int wrappedHeight(Component text, int width) {
		Font f = font();
		return f.split(text, Math.max(10, width)).size() * (f.lineHeight + 1);
	}

	public static Font font() {
		return Minecraft.getInstance().font;
	}

	/** The Stabled mark (square logo) scaled into a box. */
	public static void mark(GuiGraphicsExtractor g, int x, int y, int size) {
		g.blit(RenderPipelines.GUI_TEXTURED, MARK, x, y, 0F, 0F, size, size, MARK_SIZE, MARK_SIZE, MARK_SIZE, MARK_SIZE);
	}

	/** The Stabled wordmark (mark + name) scaled to a given height, keeping its aspect ratio. */
	public static int logo(GuiGraphicsExtractor g, int x, int y, int height, int color) {
		int w = Math.round(height * (LOGO_W / (float) LOGO_H));
		Identifier tex = theme().light() && !ThemeManager.isCustomSelected() ? LOGO_DARK_TEXT : LOGO;
		g.blit(RenderPipelines.GUI_TEXTURED, tex, x, y, 0F, 0F, w, height, LOGO_W, LOGO_H, LOGO_W, LOGO_H, color);
		return w;
	}

	/** Small check mark used for selected states, drawn with fills so it scales crisply. */
	public static void check(GuiGraphicsExtractor g, int x, int y, int color) {
		g.fill(x, y + 3, x + 2, y + 5, color);
		g.fill(x + 2, y + 4, x + 3, y + 6, color);
		g.fill(x + 3, y + 3, x + 4, y + 5, color);
		g.fill(x + 4, y + 2, x + 5, y + 4, color);
		g.fill(x + 5, y + 1, x + 6, y + 3, color);
		g.fill(x + 6, y, x + 7, y + 2, color);
	}
}
