package stabled.client.ui.widget;

import java.util.function.BooleanSupplier;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** One segment of a segmented control (radio-style choice). */
public class ChoiceButton extends BaseWidget {
	private final BooleanSupplier selected;
	private final Runnable onSelect;

	public ChoiceButton(int x, int y, int w, int h, Component label, BooleanSupplier selected, Runnable onSelect) {
		super(x, y, w, h, label);
		this.selected = selected;
		this.onSelect = onSelect;
	}

	@Override
	protected void activate() {
		this.onSelect.run();
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		boolean sel = this.selected.getAsBoolean();
		boolean hot = this.isHoveredOrFocused() && this.active;
		int x = this.getX(), y = this.getY(), w = this.getWidth(), h = this.getHeight();
		int fill = sel ? Ui.alpha(t.accent(), 0.9F) : Ui.surface(hot ? t.surfaceHover() : t.surfaceAlt());
		Ui.roundRect(g, x, y, w, h, 3, fill);
		Ui.roundOutline(g, x, y, w, h, 3, sel ? Ui.mix(t.accent(), 0xFFFFFFFF, 0.25F) : Ui.outline(hot ? t.accent() : t.border()));
		if (this.isFocused()) {
			Ui.roundOutline(g, x - 1, y - 1, w + 2, h + 2, 4, Ui.alpha(t.accent(), 0.9F));
		}
		Component label = this.getMessage();
		int maxW = w - 6;
		int tw = Math.min(Ui.font().width(label), maxW);
		Ui.textClipped(g, label, x + (w - tw) / 2, y + (h - 8) / 2, maxW, sel ? t.accentText() : (this.active ? t.text() : t.textMuted()));
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.translatable(
			this.selected.getAsBoolean() ? "stabled.narration.choice_selected" : "stabled.narration.choice", this.getMessage()));
	}
}
