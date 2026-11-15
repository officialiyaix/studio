package stabled.client.ui.widget;

import java.util.function.IntConsumer;
import java.util.function.IntFunction;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractSliderButton;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;
import org.jspecify.annotations.Nullable;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/**
 * Integer slider with a label, a live value label and keyboard support (arrow keys, inherited from vanilla).
 * {@code onChange} fires continuously while dragging; {@code onCommit} fires when the drag ends or a key is used.
 */
public class SliderWidget extends AbstractSliderButton implements Clip {
	private final Component label;
	private final int min;
	private final int max;
	private final int step;
	private final IntFunction<Component> format;
	private final IntConsumer onChange;
	private final IntConsumer onCommit;
	private @Nullable ScreenRectangle clip;

	public SliderWidget(int x, int y, int w, int h, Component label, int min, int max, int step, int initial,
		IntFunction<Component> format, IntConsumer onChange, IntConsumer onCommit) {
		super(x, y, w, h, Component.empty(), (initial - min) / (double) (max - min));
		this.label = label;
		this.min = min;
		this.max = max;
		this.step = Math.max(1, step);
		this.format = format;
		this.onChange = onChange;
		this.onCommit = onCommit;
		this.updateMessage();
	}

	public int intValue() {
		int raw = (int) Math.round(this.min + this.value * (this.max - this.min));
		int snapped = this.min + Math.round((raw - this.min) / (float) this.step) * this.step;
		return Math.max(this.min, Math.min(this.max, snapped));
	}

	@Override
	protected void updateMessage() {
		this.setMessage(Component.empty().append(this.label).append(": ").append(this.format.apply(this.intValue())));
	}

	@Override
	protected void applyValue() {
		this.onChange.accept(this.intValue());
		if (!this.isDragging()) {
			this.onCommit.accept(this.intValue());
		}
	}

	private boolean dragging;

	private boolean isDragging() {
		return this.dragging;
	}

	@Override
	public void onClick(MouseButtonEvent event, boolean doubleClick) {
		this.dragging = true;
		super.onClick(event, doubleClick);
	}

	@Override
	public void onRelease(MouseButtonEvent event) {
		super.onRelease(event);
		if (this.dragging) {
			this.dragging = false;
			this.onCommit.accept(this.intValue());
		}
	}

	@Override
	public void extractWidgetRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		int x = this.getX(), y = this.getY(), w = this.getWidth(), h = this.getHeight();
		boolean hot = this.isHoveredOrFocused() && this.active;
		if (hot) {
			Ui.roundRect(g, x, y, w, h, 3, Ui.surface(t.surfaceHover()));
		}
		if (this.isFocused()) {
			Ui.roundOutline(g, x, y, w, h, 3, Ui.alpha(t.accent(), 0.9F));
		}
		Component valueText = this.format.apply(this.intValue());
		int vw = Ui.font().width(valueText);
		boolean on = this.active;
		int accent = on ? t.accent() : Ui.mix(t.textMuted(), t.surfaceAlt(), 0.4F);
		Ui.textClipped(g, this.label, x + 4, y + 2, w - vw - 14, on ? t.text() : t.textMuted());
		Ui.text(g, valueText, x + w - vw - 4, y + 2, on ? t.accent() : t.textMuted());
		int trackX = x + 4, trackW = w - 8, trackY = y + h - 6;
		Ui.roundRect(g, trackX, trackY, trackW, 3, 1, Ui.mix(t.surfaceAlt(), t.border(), 0.7F));
		int filled = (int) Math.round(trackW * this.value);
		Ui.roundRect(g, trackX, trackY, Math.max(2, filled), 3, 1, accent);
		int kx = trackX + filled - 3;
		Ui.roundRect(g, kx, trackY - 3, 7, 9, 3, !on ? accent : hot ? Ui.mix(t.accent(), 0xFFFFFFFF, 0.3F) : t.text());
		this.handleCursor(g);
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
