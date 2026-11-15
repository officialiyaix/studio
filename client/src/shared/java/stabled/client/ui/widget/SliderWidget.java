package stabled.client.ui.widget;

import java.util.function.IntConsumer;
import java.util.function.IntFunction;
import java.util.function.IntUnaryOperator;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/**
 * Integer slider with a label and live value. Mouse drag and the arrow/Home/End keys change it;
 * {@code onChange} fires continuously, {@code onCommit} when a drag ends or a key changes the value.
 */
public class SliderWidget extends BaseWidget {
	private final Component label;
	private final int min;
	private final int max;
	private final int step;
	private final IntFunction<Component> format;
	private final IntConsumer onChange;
	private final IntConsumer onCommit;
	private int value;
	private boolean dragging;

	public SliderWidget(int x, int y, int w, int h, Component label, int min, int max, int step, int initial,
		IntFunction<Component> format, IntConsumer onChange, IntConsumer onCommit) {
		super(x, y, w, h, label);
		this.label = label;
		this.min = min;
		this.max = max;
		this.step = Math.max(1, step);
		this.format = format;
		this.onChange = onChange;
		this.onCommit = onCommit;
		this.value = this.snap(initial);
	}

	/**
	 * A slider shown as 0–100% across its whole travel, whatever the stored range is (e.g. transparency is stored
	 * as 10–90): 0% is the minimum, 100% the maximum, in steps of 1% like the launcher's sliders, so both show the same number.
	 * Callbacks receive the stored value.
	 */
	public static SliderWidget percentOfRange(int x, int y, int w, int h, Component label, int min, int max, int value,
		IntConsumer onChange, IntConsumer onCommit) {
		IntUnaryOperator toStored = p -> Math.round(min + (max - min) * p / 100F);
		int initial = Math.round((value - min) * 100F / Math.max(1, max - min));
		return new SliderWidget(x, y, w, h, label, 0, 100, 1, initial, p -> Component.literal(p + "%"),
			p -> onChange.accept(toStored.applyAsInt(p)), p -> onCommit.accept(toStored.applyAsInt(p)));
	}

	public int intValue() {
		return this.value;
	}

	private int snap(int raw) {
		int snapped = this.min + Math.round((raw - this.min) / (float) this.step) * this.step;
		return Math.max(this.min, Math.min(this.max, snapped));
	}

	private void set(int v) {
		v = this.snap(v);
		if (v != this.value) {
			this.value = v;
			this.onChange.accept(v);
		}
	}

	private void fromMouse(double mouseX) {
		double t = (mouseX - (this.getX() + 4)) / Math.max(1, this.getWidth() - 8);
		t = Math.max(0, Math.min(1, t));
		this.set((int) Math.round(this.min + t * (this.max - this.min)));
	}

	@Override
	protected void pressed(double mouseX, double mouseY) {
		this.dragging = true;
		this.fromMouse(mouseX);
	}

	@Override
	protected void dragged(double mouseX, double mouseY) {
		if (this.dragging) {
			this.fromMouse(mouseX);
		}
	}

	@Override
	protected void released(double mouseX, double mouseY) {
		if (this.dragging) {
			this.dragging = false;
			this.onCommit.accept(this.value);
		}
	}

	@Override
	protected boolean navigate(Nav nav) {
		int before = this.value;
		switch (nav) {
			case LEFT, DOWN -> this.set(this.value - this.step);
			case RIGHT, UP -> this.set(this.value + this.step);
			case HOME -> this.set(this.min);
			case END -> this.set(this.max);
		}
		if (this.value != before) {
			this.onCommit.accept(this.value);
		}
		return true;
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		int x = this.getX(), y = this.getY(), w = this.getWidth(), h = this.getHeight();
		boolean hot = this.isHoveredOrFocused() && this.active;
		if (hot) {
			Ui.roundRect(g, x, y, w, h, 3, Ui.surface(t.surfaceHover()));
		}
		if (this.isFocused()) {
			Ui.roundOutline(g, x, y, w, h, 3, Ui.alpha(t.accent(), 0.9F));
		}
		Component valueText = this.format.apply(this.value);
		int vw = Ui.font().width(valueText);
		boolean on = this.active;
		int accent = on ? t.accent() : Ui.mix(t.textMuted(), t.surfaceAlt(), 0.4F);
		Ui.textClipped(g, this.label, x + 4, y + 2, w - vw - 14, on ? t.text() : t.textMuted());
		Ui.text(g, valueText, x + w - vw - 4, y + 2, on ? t.accent() : t.textMuted());
		int trackX = x + 4, trackW = w - 8, trackY = y + h - 6;
		Ui.roundRect(g, trackX, trackY, trackW, 3, 1, Ui.mix(t.surfaceAlt(), t.border(), 0.7F));
		double frac = (this.value - this.min) / (double) (this.max - this.min);
		int filled = (int) Math.round(trackW * frac);
		Ui.roundRect(g, trackX, trackY, Math.max(2, filled), 3, 1, accent);
		int kx = trackX + filled - 3;
		Ui.roundRect(g, kx, trackY - 3, 7, 9, 3, !on ? accent : hot ? Ui.mix(t.accent(), 0xFFFFFFFF, 0.3F) : t.text());
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.empty().append(this.label).append(": ").append(this.format.apply(this.value)));
	}
}
