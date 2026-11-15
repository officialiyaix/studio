package stabled.client.ui.widget;

import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Gallery;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** One background inside a pre-theme category; choosing it makes it the Custom Background. */
public class GalleryImageTile extends BaseWidget {
	private final String category;
	private final String image;
	private final Runnable onChoose;

	public GalleryImageTile(int x, int y, int w, int h, String category, String image, Component name, Runnable onChoose) {
		super(x, y, w, h, name);
		this.category = category;
		this.image = image;
		this.onChoose = onChoose;
	}

	@Override
	protected void activate() {
		if (Gallery.applying() == null) {
			this.onChoose.run();
		}
	}

	private boolean isSelected() {
		return Gallery.key(this.category, this.image).equals(Gallery.selectedKey());
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		boolean hot = this.isHoveredOrFocused() && Gallery.applying() == null;
		boolean sel = this.isSelected();
		int x = this.getX(), y = this.getY() - (hot ? 1 : 0), w = this.getWidth(), h = this.getHeight();

		Gallery.Tex tex = Gallery.texture(this.category, this.image, false);
		if (tex != null) {
			Ui.coverBlit(g, tex.id(), x + 1, y + 1, w - 2, h - 2, tex.width(), tex.height(), 0.5, 0.4, 0xFFFFFFFF);
		} else {
			g.fill(x + 1, y + 1, x + w - 1, y + h - 1, Ui.alpha(t.surfaceAlt(), 0.9F));
			Ui.centered(g, Component.literal("…"), x + w / 2, y + h / 2 - 4, t.textMuted());
		}
		if (Gallery.key(this.category, this.image).equals(Gallery.applying())) {
			g.fill(x + 1, y + 1, x + w - 1, y + h - 1, 0x88000000);
			Ui.centered(g, Component.translatable("stabled.gallery.applying"), x + w / 2, y + h / 2 - 4, 0xFFFFFFFF);
		}
		if (sel) {
			Ui.roundRect(g, x + w - 13, y + 3, 10, 10, 5, t.accent());
			Ui.check(g, x + w - 11, y + 5, t.accentText());
		}
		int outline = sel ? t.accent() : (hot ? Ui.mix(t.border(), t.accent(), 0.6F) : t.border());
		Ui.roundOutline(g, x, y, w, h, 3, outline);
		if (sel) {
			Ui.roundOutline(g, x + 1, y + 1, w - 2, h - 2, 2, Ui.alpha(t.accent(), 0.55F));
		}
		if (this.isFocused()) {
			Ui.roundOutline(g, x - 1, y - 1, w + 2, h + 2, 4, Ui.alpha(t.accent(), 0.9F));
		}
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.translatable(this.isSelected() ? "stabled.narration.choice_selected" : "stabled.narration.choice", this.getMessage()));
	}
}
