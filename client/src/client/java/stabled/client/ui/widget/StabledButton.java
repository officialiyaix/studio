package stabled.client.ui.widget;

import java.util.function.Consumer;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.input.InputWithModifiers;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** Themed push button. */
public class StabledButton extends AbstractButton implements Clip {
	public enum Style { PRIMARY, SECONDARY, CTA, GHOST }

	private final Style style;
	private final Consumer<StabledButton> onPress;
	private @Nullable ScreenRectangle clip;

	public StabledButton(int x, int y, int w, int h, Component label, Style style, Consumer<StabledButton> onPress) {
		super(x, y, w, h, label);
		this.style = style;
		this.onPress = onPress;
	}

	@Override
	public void onPress(InputWithModifiers input) {
		this.onPress.accept(this);
	}

	@Override
	protected void extractContents(GuiGraphicsExtractor g, int mouseX, int mouseY, float a) {
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
		// Primary actions keep their colour solid on glass so they stay the strongest element.
		Ui.roundRect(g, x, y, w, h, 3, fill);
		Ui.roundOutline(g, x, y, w, h, 3, border);
		if (this.isFocused()) {
			Ui.roundOutline(g, x - 1, y - 1, w + 2, h + 2, 4, Ui.alpha(t.accent(), 0.9F));
		}
		Component label = this.getMessage();
		int tw = Ui.font().width(label);
		int maxW = w - 8;
		int tx = x + Math.max(4, (w - Math.min(tw, maxW)) / 2);
		Ui.textClipped(g, label, tx, y + (h - 8) / 2, maxW, text);
		this.handleCursor(g);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		this.defaultButtonNarrationText(output);
	}

	@Override
	public void setClip(@Nullable ScreenRectangle clip) {
		this.clip = clip;
	}

	@Override
	public boolean isMouseOver(double mouseX, double mouseY) {
		return super.isMouseOver(mouseX, mouseY) && Clip.allows(this.clip, mouseX, mouseY);
	}
}
