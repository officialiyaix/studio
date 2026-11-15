package stabled.client.ui.widget;

import java.util.function.BooleanSupplier;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.input.InputWithModifiers;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** One segment of a segmented control (radio-style choice). */
public class ChoiceButton extends AbstractButton implements Clip {
	private final BooleanSupplier selected;
	private final Runnable onSelect;
	private @Nullable ScreenRectangle clip;

	public ChoiceButton(int x, int y, int w, int h, Component label, BooleanSupplier selected, Runnable onSelect) {
		super(x, y, w, h, label);
		this.selected = selected;
		this.onSelect = onSelect;
	}

	@Override
	public void onPress(InputWithModifiers input) {
		this.onSelect.run();
	}

	@Override
	protected void extractContents(GuiGraphicsExtractor g, int mouseX, int mouseY, float a) {
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
		this.handleCursor(g);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.translatable(
			this.selected.getAsBoolean() ? "stabled.narration.choice_selected" : "stabled.narration.choice", this.getMessage()));
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
