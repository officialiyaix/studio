package stabled.client.hud;

import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.screens.ChatScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.world.effect.MobEffectInstance;
import stabled.client.StabledClient;
import stabled.client.config.ClientSettings;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/**
 * The FPS display. The number is {@link Minecraft#getFps()}: the renderer's own count of frames presented during
 * the last second — the same value the debug screen shows. It is drawn as a HUD element (so F1 hides it) and is
 * placed in a corner away from the crosshair, with offsets that avoid effect icons and the chat input.
 */
public final class FpsHud {
	private static final int PAD_X = 4;
	private static final int PAD_Y = 3;

	private FpsHud() {
	}

	public static void extract(GuiGraphicsExtractor g, DeltaTracker delta) {
		ClientSettings s = StabledClient.settings();
		Minecraft mc = Minecraft.getInstance();
		if (!s.fpsEnabled || mc.gui.hud.isHidden() || mc.debugEntries.isOverlayVisible()) {
			return;
		}
		boolean bottomLeft = "bottom-left".equals(s.fpsPosition);
		if (bottomLeft && mc.gui.screen() instanceof ChatScreen) {
			return; // The chat input occupies that corner while typing.
		}
		draw(g, Component.translatable("stabled.fps.value", mc.getFps()), s.fpsPosition, (float) s.fpsScale, s.fpsColor, s.fpsBackground,
			g.guiWidth(), g.guiHeight(), effectOffset(mc, s.fpsPosition));
	}

	/** Shared by the HUD and labelled previews so both look identical. */
	public static void draw(GuiGraphicsExtractor g, Component text, String position, float scale, String color, String background,
		int areaWidth, int areaHeight, int topRightOffset) {
		Font font = Ui.font();
		int textW = font.width(text);
		int boxW = textW + PAD_X * 2;
		int boxH = font.lineHeight + PAD_Y * 2 - 1;
		float sw = boxW * scale;
		float sh = boxH * scale;
		int margin = 4;
		float x;
		float y;
		switch (position) {
			case "top-right" -> {
				x = areaWidth - sw - margin;
				y = margin + topRightOffset;
			}
			case "bottom-left" -> {
				x = margin;
				y = areaHeight - sh - margin;
			}
			case "bottom-right" -> {
				x = areaWidth - sw - margin;
				y = areaHeight - sh - margin;
			}
			default -> {
				x = margin;
				y = margin;
			}
		}
		Theme t = Ui.theme();
		g.pose().pushMatrix();
		g.pose().translate(x, y);
		g.pose().scale(scale, scale);
		boolean shadow = "shadow".equals(background);
		if ("panel".equals(background)) {
			Ui.roundRect(g, 0, 0, boxW, boxH, 3, Ui.alpha(t.light() ? 0xFFFFFFFF : 0xFF000000, 0.55F));
			Ui.roundOutline(g, 0, 0, boxW, boxH, 3, Ui.alpha(t.border(), 0.8F));
		}
		int c = switch (color) {
			case "white" -> 0xFFFFFFFF;
			case "green" -> 0xFF4ADE80;
			case "yellow" -> 0xFFFACC15;
			case "cyan" -> 0xFF22D3EE;
			default -> t.light() && "panel".equals(background) ? t.accent() : Ui.mix(t.accent(), 0xFFFFFFFF, 0.25F);
		};
		g.text(font, text, PAD_X, PAD_Y, Ui.opaque(c), shadow);
		g.pose().popMatrix();
	}

	/** Vanilla draws status-effect icons in the top-right corner: one row per category. */
	private static int effectOffset(Minecraft mc, String position) {
		if (!"top-right".equals(position) || mc.player == null) {
			return 0;
		}
		boolean beneficial = false;
		boolean harmful = false;
		for (MobEffectInstance effect : mc.player.getActiveEffects()) {
			if (!effect.showIcon()) {
				continue;
			}
			if (effect.getEffect().value().isBeneficial()) {
				beneficial = true;
			} else {
				harmful = true;
			}
		}
		return (beneficial ? 26 : 0) + (harmful ? 26 : 0);
	}
}
