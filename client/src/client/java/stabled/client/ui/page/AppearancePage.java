package stabled.client.ui.page;

import java.util.ArrayList;
import java.util.List;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import stabled.client.config.Appearance;
import stabled.client.theme.Theme;
import stabled.client.theme.ThemeManager;
import stabled.client.ui.Ui;
import stabled.client.ui.widget.ChoiceButton;
import stabled.client.ui.widget.DrawWidget;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.SliderWidget;
import stabled.client.ui.widget.StabledButton;
import stabled.client.ui.widget.ThemeTile;
import stabled.client.ui.widget.ToggleWidget;

/** Step 1 / Appearance: all nine themes, then Simple or Frosted Glass, then the transparency slider and samples. */
public final class AppearancePage implements Page {
	private boolean sampleToggle = true;

	@Override
	public Component title() {
		return Component.translatable("stabled.page.appearance");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.appearance.summary");
	}

	@Override
	public void build(PageBuilder b) {
		List<String> ids = new ArrayList<>();
		for (Theme t : ThemeManager.builtInThemes()) {
			ids.add(t.id());
		}
		ids.add(ThemeManager.CUSTOM_ID);
		int columns = b.width() >= 300 ? 3 : 2;
		int tileH = b.width() >= 300 ? 36 : 32;
		for (int row = 0; row * columns < ids.size(); row++) {
			int start = row * columns;
			int count = Math.min(columns, ids.size() - start);
			int gap = 5;
			int cellW = (b.width() - gap * (columns - 1)) / columns;
			for (int i = 0; i < count; i++) {
				String id = ids.get(start + i);
				b.add(new ThemeTile(b.x() + i * (cellW + gap), b.y(), cellW, tileH, id, () -> {
					ThemeManager.select(id);
					b.rebuild().run();
				}));
			}
			b.advance(tileH + gap);
		}
		if (ThemeManager.isCustomSelected()) {
			String error = ThemeManager.backgroundError();
			if (error != null) {
				b.label(Component.literal(error), LabelWidget.Tone.DANGER);
			} else if (!ThemeManager.hasCustomImage()) {
				b.label(Component.translatable("stabled.theme.custom_missing"), LabelWidget.Tone.WARNING);
			}
		}
		b.space(4);
		b.heading(Component.translatable("stabled.surface.heading"));
		Appearance a = ThemeManager.appearance();
		b.row(2, PageBuilder.ROW, 4, (i, x, y, w, h) -> {
			String value = i == 0 ? Appearance.SURFACE_SIMPLE : Appearance.SURFACE_GLASS;
			return new ChoiceButton(x, y, w, h, Component.translatable("stabled.surface." + value),
				() -> value.equals(ThemeManager.appearance().surface), () -> {
					ThemeManager.setSurface(value);
					b.rebuild().run();
				});
		});
		b.label(Component.translatable(a.isGlass() ? "stabled.surface.glass.desc" : "stabled.surface.simple.desc"), LabelWidget.Tone.MUTED);
		if (a.isGlass()) {
			b.add(new SliderWidget(b.x(), b.y(), b.width(), 22, Component.translatable("stabled.surface.transparency"),
				Appearance.MIN_TRANSPARENCY, Appearance.MAX_TRANSPARENCY, 5, a.glassTransparency,
				v -> Component.literal(v + "%"),
				v -> ThemeManager.setTransparency(v, false),
				v -> ThemeManager.setTransparency(v, true)));
			b.advance(26);
		}
		this.preview(b);
		b.label(Component.translatable("stabled.appearance.global"), LabelWidget.Tone.MUTED);
	}

	/** Live samples drawn over a strip of the actual background so translucency is visible. */
	private void preview(PageBuilder b) {
		int h = 46;
		int x = b.x();
		int y = b.y();
		int w = b.width();
		b.add(new DrawWidget(x, y, w, h, Component.translatable("stabled.surface.preview"), (g, px, py, pw, ph, pt) -> {
			Theme t = Ui.theme();
			g.enableScissor(px, py, px + pw, py + ph);
			Identifier tex = ThemeManager.isCustomSelected() ? ThemeManager.customBackgroundTexture() : null;
			if (tex != null) {
				Ui.coverBlit(g, tex, px, py, pw, ph, ThemeManager.backgroundWidth(), ThemeManager.backgroundHeight());
			} else {
				g.fillGradient(px, py, px + pw, py + ph, t.bgAlt(), t.bg());
				// Diagonal stripes make transparency easy to judge.
				for (int sx = -ph; sx < pw; sx += 10) {
					for (int k = 0; k < ph; k += 2) {
						g.fill(px + sx + k, py + k, px + sx + k + 4, py + k + 2, Ui.alpha(t.accent(), 0.22F));
					}
				}
			}
			g.disableScissor();
			Ui.roundOutline(g, px, py, pw, ph, 3, t.border());
			int panelX = px + pw - Math.min(110, pw / 3) - 6;
			Ui.panel(g, panelX, py + 6, Math.min(110, pw / 3), ph - 12, 3);
			Ui.textClipped(g, Component.translatable("stabled.surface.sample_panel"), panelX + 6, py + 12, Math.min(110, pw / 3) - 12, t.text());
			Ui.textClipped(g, Component.translatable("stabled.surface.sample_text"), panelX + 6, py + 24, Math.min(110, pw / 3) - 12, t.textMuted());
		}));
		int sampleW = Math.max(40, (w - Math.min(110, w / 3) - 30) / 3);
		b.add(new StabledButton(x + 6, y + 13, sampleW, 20, Component.translatable("stabled.surface.sample_primary"), StabledButton.Style.PRIMARY, btn -> {
		}));
		b.add(new StabledButton(x + 10 + sampleW, y + 13, sampleW, 20, Component.translatable("stabled.surface.sample_secondary"), StabledButton.Style.SECONDARY, btn -> {
		}));
		b.add(new ToggleWidget(x + 14 + sampleW * 2, y + 13, sampleW, 20, Component.translatable("stabled.surface.sample_toggle"),
			() -> this.sampleToggle, v -> this.sampleToggle = v));
		b.advance(h + 4);
	}
}
