package stabled.client.ui.widget;

import java.util.function.Consumer;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** Themed push button. */
public class StabledButton extends BaseWidget {
	public enum Style { PRIMARY, SECONDARY, CTA, GHOST }

	private final Style style;
	private final Consumer<StabledButton> onPress;

	public StabledButton(int x, int y, int w, int h, Component label, Style style, Consumer<StabledButton> onPress) {
		super(x, y, w, h, label);
		this.style = style;
		this.onPress = onPress;
	}

	@Override
	protected void activate() {
		this.onPress.accept(this);
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		boolean hot = this.isHoveredOrFocused() && this.active;
		int x = this.getX(), y = this.getY(), w = this.getWidth(), h = this.getHeight();
		int fill;
		int text;
		int border;
		switch (this.style) {
			case PRIMARY -> {
				fill = hot ? Ui.mix(t.accent(), 0xFFFFFFFF, 0.12F) : t.accent();
				text = t.accentText();
				border = Ui.mix(t.accent(), 0xFFFFFFFF, 0.25F);
			}
			case CTA -> {
				fill = hot ? Ui.mix(t.cta(), 0xFFFFFFFF, 0.12F) : t.cta();
				text = t.ctaText();
				border = Ui.mix(t.cta(), 0xFFFFFFFF, 0.25F);
			}
			case GHOST -> {
				fill = hot ? Ui.surface(t.surfaceHover()) : 0;
				text = hot ? t.text() : t.textMuted();
				border = hot ? Ui.outline(t.border()) : 0;
			}
			default -> {
				fill = Ui.surface(hot ? t.surfaceHover() : t.surfaceAlt());
				text = t.text();
				border = Ui.outline(hot ? t.accent() : t.border());
			}
		}
		if (!this.active) {
			fill = Ui.alpha(fill, 0.45F);
			text = t.textMuted();
		}
		Ui.roundRect(g, x, y, w, h, 3, fill);
		Ui.roundOutline(g, x, y, w, h, 3, border);
		if (this.isFocused()) {
			Ui.roundOutline(g, x - 1, y - 1, w + 2, h + 2, 4, Ui.alpha(t.accent(), 0.9F));
		}
		Component label = this.getMessage();
		int maxW = w - 8;
		int tw = Ui.font().width(label);
		int tx = x + Math.max(4, (w - Math.min(tw, maxW)) / 2);
		Ui.textClipped(g, label, tx, y + (h - 8) / 2, maxW, text);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		this.defaultButtonNarrationText(output);
	}
}
