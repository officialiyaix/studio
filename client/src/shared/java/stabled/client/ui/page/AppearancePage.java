package stabled.client.ui.page;

import java.util.ArrayList;
import java.util.List;
import net.minecraft.network.chat.Component;
import stabled.client.config.Appearance;
import stabled.client.theme.Gallery;
import stabled.client.theme.Theme;
import stabled.client.theme.ThemeManager;
import stabled.client.ui.Ui;
import stabled.client.ui.widget.ChoiceButton;
import stabled.client.ui.widget.DrawWidget;
import stabled.client.ui.widget.GalleryCategoryTile;
import stabled.client.ui.widget.GalleryImageTile;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.SliderWidget;
import stabled.client.ui.widget.StabledButton;
import stabled.client.ui.widget.ThemeTile;
import stabled.client.ui.widget.ToggleWidget;

/**
 * Step 1 / Appearance: pre-themes (bundled backgrounds by category), a separator, the basic themes, then Simple or
 * Frosted Glass, the transparency slider and samples.
 */
public final class AppearancePage implements Page {
	private boolean sampleToggle = true;
	/** Pre-theme category being browsed (null = the category list). Kept while the page is rebuilt. */
	private String openCategory;
	private String galleryError;

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
		this.preThemes(b);
		this.separator(b);
		this.basicThemes(b);
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
			b.add(SliderWidget.percentOfRange(b.x(), b.y(), b.width(), 22, Component.translatable("stabled.surface.transparency"),
				Appearance.MIN_TRANSPARENCY, Appearance.MAX_TRANSPARENCY, a.glassTransparency,
				v -> ThemeManager.setTransparency(v, false),
				v -> ThemeManager.setTransparency(v, true)));
			b.advance(26);
		}
		this.preview(b);
		b.label(Component.translatable("stabled.appearance.global"), LabelWidget.Tone.MUTED);
	}

	/** Pre-themes: the backgrounds that ship with Stabled, by category (a blurred peek of two images each). */
	private void preThemes(PageBuilder b) {
		b.heading(Component.translatable("stabled.themes.pre"));
		List<Gallery.Category> categories = Gallery.categories();
		if (categories.isEmpty()) {
			b.label(Component.translatable("stabled.gallery.unavailable"), LabelWidget.Tone.MUTED);
			return;
		}
		Gallery.Category open = null;
		for (Gallery.Category c : categories) {
			if (c.id().equals(this.openCategory)) {
				open = c;
			}
		}
		int gap = 5;
		if (open == null) {
			b.label(Component.translatable("stabled.themes.pre.desc"), LabelWidget.Tone.MUTED);
			int columns = b.width() >= 300 ? 3 : 2;
			int cellW = (b.width() - gap * (columns - 1)) / columns;
			int tileH = Math.max(40, Math.round(cellW * 0.5F));
			for (int i = 0; i < categories.size(); i++) {
				Gallery.Category c = categories.get(i);
				int col = i % columns;
				b.add(new GalleryCategoryTile(b.x() + col * (cellW + gap), b.y(), cellW, tileH, c, () -> {
					this.openCategory = c.id();
					this.galleryError = null;
					b.rebuild().run();
				}));
				if (col == columns - 1 || i == categories.size() - 1) {
					b.advance(tileH + gap);
				}
			}
		} else {
			Gallery.Category category = open;
			int backW = Math.min(110, b.width() / 3);
			b.add(new StabledButton(b.x(), b.y(), backW, 18, Component.literal("‹ ").append(Component.translatable("stabled.gallery.back")),
				StabledButton.Style.SECONDARY, btn -> {
					this.openCategory = null;
					this.galleryError = null;
					b.rebuild().run();
				}));
			b.add(new LabelWidget(b.x() + backW + 8, b.y() + 5, b.width() - backW - 8,
				() -> Component.literal(category.name()).append(" · ").append(String.valueOf(category.images().size())), LabelWidget.Tone.HEADING));
			b.advance(24);
			int columns = b.width() >= 300 ? 3 : 2;
			int cellW = (b.width() - gap * (columns - 1)) / columns;
			int tileH = Math.round(cellW * 9F / 16F);
			for (int i = 0; i < category.images().size(); i++) {
				String image = category.images().get(i);
				int col = i % columns;
				b.add(new GalleryImageTile(b.x() + col * (cellW + gap), b.y(), cellW, tileH, category.id(), image,
					Component.translatable("stabled.gallery.image", category.name(), i + 1),
					() -> Gallery.apply(category.id(), image, () -> b.rebuild().run(), message -> {
						this.galleryError = message;
						b.rebuild().run();
					})));
				if (col == columns - 1 || i == category.images().size() - 1) {
					b.advance(tileH + gap);
				}
			}
		}
		if (this.galleryError != null) {
			b.label(Component.literal(this.galleryError), LabelWidget.Tone.DANGER);
		}
	}

	/** A thin rule between the pre-themes and the basic themes. */
	private void separator(PageBuilder b) {
		b.space(2);
		b.add(new DrawWidget(b.x(), b.y(), b.width(), 5, Component.empty(), (g, px, py, pw, ph, pt) -> {
			Theme t = Ui.theme();
			g.fill(px + 8, py + 2, px + pw - 8, py + 3, Ui.alpha(t.border(), 0.9F));
		}));
		b.advance(9);
	}

	/** Basic themes: the eight built-in themes and the Custom Background slot. */
	private void basicThemes(PageBuilder b) {
		b.heading(Component.translatable("stabled.themes.basic"));
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
	}

	/** Live samples drawn over a strip of the actual background so translucency is visible. */
	private void preview(PageBuilder b) {
		int h = 46;
		int x = b.x();
		int y = b.y();
		int w = b.width();
		b.add(new DrawWidget(x, y, w, h, Component.translatable("stabled.surface.preview"), (g, px, py, pw, ph, pt) -> {
			Theme t = Ui.theme();
			g.scissor(px, py, px + pw, py + ph);
			String tex = ThemeManager.isCustomSelected() ? ThemeManager.customBackgroundTexture() : null;
			if (tex != null) {
				Ui.coverBlit(g, tex, px, py, pw, ph, ThemeManager.backgroundWidth(), ThemeManager.backgroundHeight());
			} else {
				g.gradient(px, py, px + pw, py + ph, t.bgAlt(), t.bg());
				// Diagonal stripes make transparency easy to judge.
				for (int sx = -ph; sx < pw; sx += 10) {
					for (int k = 0; k < ph; k += 2) {
						g.fill(px + sx + k, py + k, px + sx + k + 4, py + k + 2, Ui.alpha(t.accent(), 0.22F));
					}
				}
			}
			g.endScissor();
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
