package stabled.client.ui.widget;

import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Gallery;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** A pre-theme category: a blurred peek of (up to) two of its backgrounds with its name and image count. */
public class GalleryCategoryTile extends BaseWidget {
	private final Gallery.Category category;
	private final Runnable onOpen;

	public GalleryCategoryTile(int x, int y, int w, int h, Gallery.Category category, Runnable onOpen) {
		super(x, y, w, h, Component.literal(category.name()));
		this.category = category;
		this.onOpen = onOpen;
	}

	@Override
	protected void activate() {
		this.onOpen.run();
	}

	private boolean isSelected() {
		String sel = Gallery.selectedKey();
		return sel != null && sel.startsWith(this.category.id() + "/");
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		boolean hot = this.isHoveredOrFocused();
		boolean sel = this.isSelected();
		int x = this.getX(), y = this.getY() - (hot ? 1 : 0), w = this.getWidth(), h = this.getHeight();

		g.fill(x + 1, y + 1, x + w - 1, y + h - 1, Ui.alpha(0xFF000000, 0.55F));
		int n = Math.min(2, this.category.images().size());
		g.scissor(x + 1, y + 1, x + w - 1, y + h - 1);
		if (n == 1) {
			this.peek(g, 0, x + w / 8, y + 3, w - w / 4, h - 6);
		} else {
			// Two overlapping cards; they spread apart on hover.
			int cw = (int) (w * 0.62F);
			int spread = hot ? 3 : 0;
			this.peek(g, 0, x + 3 - spread, y + 2, cw, h - 7);
			this.peek(g, 1, x + w - cw - 3 + spread, y + 5, cw, h - 7);
		}
		g.gradient(x + 1, y + h / 3, x + w - 1, y + h - 1, 0x00000000, 0xC8000000);
		g.endScissor();

		String count = String.valueOf(this.category.images().size());
		int countW = Ui.font().width(count) + 8;
		Ui.roundRect(g, x + w - countW - 4, y + h - 13, countW, 10, 3, Ui.alpha(0xFF000000, 0.6F));
		Ui.text(g, Component.literal(count), x + w - countW, y + h - 12, 0xFFFFFFFF);
		Ui.textClipped(g, this.getMessage(), x + 5, y + h - 12, w - countW - 14 - (sel ? 10 : 0), 0xFFFFFFFF);
		if (sel) {
			Ui.check(g, x + 5 + Math.min(Ui.font().width(this.getMessage()), w - countW - 24) + 3, y + h - 11, t.accent());
		}

		int outline = sel ? t.accent() : (hot ? Ui.mix(t.border(), t.accent(), 0.6F) : t.border());
		Ui.roundOutline(g, x, y, w, h, 3, outline);
		if (this.isFocused()) {
			Ui.roundOutline(g, x - 1, y - 1, w + 2, h + 2, 4, Ui.alpha(t.accent(), 0.9F));
		}
	}

	private void peek(Gfx g, int index, int x, int y, int w, int h) {
		Gallery.Tex tex = Gallery.texture(this.category.id(), this.category.images().get(index), true);
		if (tex == null) {
			Ui.roundRect(g, x, y, w, h, 3, Ui.alpha(Ui.theme().surfaceAlt(), 0.8F));
			return;
		}
		Ui.coverBlit(g, tex.id(), x, y, w, h, tex.width(), tex.height(), 0.5, 0.4, 0xFFFFFFFF);
		Ui.roundOutline(g, x, y, w, h, 2, 0x40000000);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.translatable("stabled.gallery.category.narration", this.getMessage(), this.category.images().size()));
	}
}
