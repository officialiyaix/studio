package stabled.client.ui.page;

import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.config.ClientSettings;
import stabled.client.hud.FpsHud;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;
import stabled.client.ui.widget.ChoiceButton;
import stabled.client.ui.widget.DrawWidget;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.ToggleWidget;

/** Step 3 / FPS Display: toggle, placement, size, colour, background and a labelled position preview. */
public final class FpsPage implements Page {
	@Override
	public Component title() {
		return Component.translatable("stabled.page.fps");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.fps.summary");
	}

	@Override
	public void build(PageBuilder b) {
		ClientSettings s = StabledClient.settings();
		b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("stabled.fps.toggle"),
			() -> StabledClient.settings().fpsEnabled, v -> StabledClient.update(x -> x.fpsEnabled = v)));
		b.advance(PageBuilder.ROW + 6);
		choices(b, "stabled.fps.position", ClientSettings.FPS_POSITIONS, () -> StabledClient.settings().fpsPosition,
			v -> StabledClient.update(x -> x.fpsPosition = v));
		b.heading(Component.translatable("stabled.fps.size"));
		b.row(ClientSettings.FPS_SCALES.size(), PageBuilder.ROW, 3, (i, x, y, w, h) -> {
			double scale = ClientSettings.FPS_SCALES.get(i);
			return new ChoiceButton(x, y, w, h, Component.literal(Math.round(scale * 100) + "%"),
				() -> StabledClient.settings().fpsScale == scale, () -> StabledClient.update(c -> c.fpsScale = scale));
		});
		choices(b, "stabled.fps.color", ClientSettings.FPS_COLORS, () -> StabledClient.settings().fpsColor,
			v -> StabledClient.update(x -> x.fpsColor = v));
		choices(b, "stabled.fps.background", ClientSettings.FPS_BACKGROUNDS, () -> StabledClient.settings().fpsBackground,
			v -> StabledClient.update(x -> x.fpsBackground = v));

		int pw = Math.min(b.width(), 220);
		int ph = Math.round(pw * 9 / 16F);
		int px = b.x() + (b.width() - pw) / 2;
		b.add(new DrawWidget(px, b.y(), pw, ph, Component.translatable("stabled.fps.preview_label"), (g, x, y, w, h, pt) -> {
			Theme t = Ui.theme();
			ClientSettings c = StabledClient.settings();
			g.fillGradient(x, y, x + w, y + h, Ui.mix(t.bgAlt(), 0xFF3A6EA5, 0.35F), Ui.mix(t.bg(), 0xFF3F7F3A, 0.35F));
			Ui.roundOutline(g, x, y, w, h, 3, t.border());
			int cx = x + w / 2, cy = y + h / 2;
			g.fill(cx - 4, cy, cx + 5, cy + 1, 0xCCFFFFFF);
			g.fill(cx, cy - 4, cx + 1, cy + 5, 0xCCFFFFFF);
			g.fill(cx - 46, y + h - 12, cx + 46, y + h - 3, 0x88000000);
			g.fill(x + 4, y + h - 30, x + 70, y + h - 16, 0x44000000);
			if (c.fpsEnabled) {
				g.enableScissor(x, y, x + w, y + h);
				g.pose().pushMatrix();
				g.pose().translate(x, y);
				FpsHud.draw(g, Component.translatable("stabled.fps.value", Minecraft.getInstance().getFps()), c.fpsPosition,
					(float) c.fpsScale * 0.75F, c.fpsColor, c.fpsBackground, w, h, 0);
				g.pose().popMatrix();
				g.disableScissor();
			} else {
				Ui.centered(g, Component.translatable("stabled.fps.hidden"), cx, y + 6, t.textMuted());
			}
		}));
		b.advance(ph + 4);
		b.label(Component.translatable("stabled.fps.preview_label"), LabelWidget.Tone.MUTED);
		b.label(Component.translatable("stabled.fps.measure"), LabelWidget.Tone.MUTED);
	}

	static void choices(PageBuilder b, String key, List<String> values, java.util.function.Supplier<String> current, java.util.function.Consumer<String> set) {
		b.heading(Component.translatable(key));
		b.row(values.size(), PageBuilder.ROW, 3, (i, x, y, w, h) -> {
			String v = values.get(i);
			return new ChoiceButton(x, y, w, h, Component.translatable(key + "." + v), () -> v.equals(current.get()), () -> set.accept(v));
		});
	}
}
