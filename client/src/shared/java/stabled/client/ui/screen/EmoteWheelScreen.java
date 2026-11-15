package stabled.client.ui.screen;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.KeyMapping;
import net.minecraft.network.chat.Component;
import stabled.client.emote.Emote;
import stabled.client.emote.EmotePlayer;
import stabled.client.platform.BaseScreen;
import stabled.client.platform.Gfx;
import stabled.client.platform.KeyInput;
import stabled.client.platform.Platform;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/**
 * The emote wheel: hold the emote key, point at an emote and let go. A quick tap keeps the wheel open so an emote
 * can be clicked instead; number keys 1–8 pick directly and Esc cancels. The game keeps running behind it.
 */
public final class EmoteWheelScreen extends BaseScreen {
	private static final long TAP_MS = 280;
	private static final Emote[] EMOTES = Emote.values();

	private final KeyMapping key;
	private final long openedAt = System.nanoTime();
	private boolean tapMode;
	private int hovered = -1;

	public EmoteWheelScreen(KeyMapping key) {
		super(Component.translatable("stabled.emotes.title"));
		this.key = key;
	}

	@Override
	public boolean isPauseScreen() {
		return false;
	}

	private float openProgress() {
		float t = Math.min(1F, (System.nanoTime() - this.openedAt) / 170_000_000F);
		return 1F - (1F - t) * (1F - t) * (1F - t);
	}

	private int radius() {
		return (int) Math.min(this.height * 0.3F, 100F);
	}

	private int hoveredAt(double mouseX, double mouseY) {
		double dx = mouseX - this.width / 2.0;
		double dy = mouseY - this.height / 2.0;
		if (dx * dx + dy * dy < 22 * 22) {
			return -1;
		}
		double angle = Math.atan2(dy, dx) + Math.PI / 2;
		int sector = (int) Math.round(angle / (Math.PI * 2 / EMOTES.length));
		return Math.floorMod(sector, EMOTES.length);
	}

	private void play(int index) {
		Platform.setScreen(null);
		if (index >= 0 && index < EMOTES.length) {
			EmotePlayer.play(EMOTES[index]);
		}
	}

	@Override
	protected void drawBackground(Gfx g, int mouseX, int mouseY, float partialTick) {
		float a = this.openProgress();
		g.fill(0, 0, this.width, this.height, Ui.alpha(0xFF000000, 0.38F * a));
	}

	@Override
	protected void drawForeground(Gfx g, int mouseX, int mouseY, float partialTick) {
		Theme t = Ui.theme();
		float a = this.openProgress();
		int cx = this.width / 2;
		int cy = this.height / 2;
		int r = Math.round(this.radius() * (0.55F + 0.45F * a));
		this.hovered = this.hoveredAt(mouseX, mouseY);

		// A faint ring and the pointer from the centre towards the mouse.
		for (int i = 0; i < 48; i++) {
			double ang = i * Math.PI * 2 / 48;
			int px = cx + (int) Math.round(Math.cos(ang) * r);
			int py = cy + (int) Math.round(Math.sin(ang) * r);
			g.fill(px - 1, py - 1, px + 1, py + 1, Ui.alpha(t.border(), 0.8F * a));
		}
		if (this.hovered >= 0) {
			double dx = mouseX - cx;
			double dy = mouseY - cy;
			double len = Math.sqrt(dx * dx + dy * dy);
			int steps = (int) Math.min(len, r - 26) / 6;
			for (int i = 2; i <= steps; i++) {
				int px = cx + (int) Math.round(dx / len * i * 6);
				int py = cy + (int) Math.round(dy / len * i * 6);
				g.fill(px - 1, py - 1, px + 2, py + 2, Ui.alpha(t.accent(), 0.35F + 0.65F * i / Math.max(1, steps)));
			}
		}

		// Centre: the emote under the pointer.
		int cs = 64;
		Ui.roundRect(g, cx - cs / 2, cy - cs / 2, cs, cs, 10, Ui.alpha(Ui.opaque(t.surface()), 0.92F * a));
		Ui.roundOutline(g, cx - cs / 2, cy - cs / 2, cs, cs, 10, Ui.alpha(this.hovered >= 0 ? t.accent() : t.border(), a));
		if (this.hovered >= 0) {
			Emote e = EMOTES[this.hovered];
			this.glyph(g, e.glyph, cx, cy - 10, 2F, t.accent());
			Ui.centered(g, e.label(), cx, cy + 12, t.text());
		} else {
			Ui.mark(g, cx - 9, cy - 16, 18);
			Ui.centered(g, Component.translatable("stabled.emotes.pick"), cx, cy + 8, t.textMuted());
		}

		// The emotes around it.
		for (int i = 0; i < EMOTES.length; i++) {
			Emote e = EMOTES[i];
			double ang = -Math.PI / 2 + i * Math.PI * 2 / EMOTES.length;
			int bx = cx + (int) Math.round(Math.cos(ang) * r);
			int by = cy + (int) Math.round(Math.sin(ang) * r);
			boolean hot = i == this.hovered;
			int s = hot ? 42 : 36;
			int bg = hot ? t.accent() : Ui.alpha(Ui.opaque(t.surface()), 0.94F);
			Ui.roundRect(g, bx - s / 2, by - s / 2, s, s, 9, Ui.alpha(bg, a));
			Ui.roundOutline(g, bx - s / 2, by - s / 2, s, s, 9, Ui.alpha(hot ? Ui.mix(t.accent(), 0xFFFFFFFF, 0.35F) : t.border(), a));
			this.glyph(g, e.glyph, bx, by - 1, hot ? 2F : 1.6F, hot ? t.accentText() : t.text());
			g.text(String.valueOf(i + 1), bx - s / 2 + 4, by - s / 2 + 3, Ui.alpha(hot ? t.accentText() : t.textMuted(), 0.8F * a), false);
			Ui.centered(g, e.label(), bx, by + s / 2 + 3, Ui.alpha(hot ? t.text() : t.textMuted(), a));
		}

		Component hint = Component.translatable(this.tapMode ? "stabled.emotes.hint_click" : "stabled.emotes.hint_hold", this.key.getTranslatedKeyMessage());
		Ui.centered(g, hint, cx, Math.min(this.height - 14, cy + r + 34), Ui.alpha(t.textMuted(), a));
	}

	/** Draws a symbol centred on (x, y) at a scale. */
	private void glyph(Gfx g, String glyph, int x, int y, float scale, int color) {
		int w = Ui.font().width(glyph);
		g.push();
		g.translate(x, y);
		g.scale(scale, scale);
		g.text(glyph, -w / 2, -4, color, false);
		g.pop();
	}

	@Override
	protected boolean keyUp(KeyInput input) {
		if (!input.matches(this.key)) {
			return false;
		}
		long held = (System.nanoTime() - this.openedAt) / 1_000_000L;
		if (this.hovered >= 0) {
			this.play(this.hovered);
		} else if (held < TAP_MS && !this.tapMode) {
			this.tapMode = true; // a tap: keep the wheel open to click
		} else {
			this.onClose();
		}
		return true;
	}

	@Override
	protected boolean keyDown(KeyInput input) {
		// Minecraft's own key constants: GLFW codes before 26.3, SDL scancodes from 26.3 (both contiguous for 1–9).
		int k = input.key();
		if (k >= InputConstants.KEY_1 && k < InputConstants.KEY_1 + EMOTES.length) {
			this.play(k - InputConstants.KEY_1);
			return true;
		}
		if (this.tapMode && input.matches(this.key)) {
			if (this.hovered >= 0) {
				this.play(this.hovered);
			} else {
				this.onClose();
			}
			return true;
		}
		// Holding the key repeats it; nothing to do.
		return input.matches(this.key);
	}

	@Override
	protected boolean clicked(double mouseX, double mouseY, int button) {
		int i = this.hoveredAt(mouseX, mouseY);
		if (button == InputConstants.MOUSE_BUTTON_LEFT && i >= 0) {
			this.play(i);
		} else {
			this.onClose();
		}
		return true;
	}
}
