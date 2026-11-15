package stabled.client.ui.page;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Supplier;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.network.chat.Component;
import stabled.client.ui.widget.LabelWidget;

/** Vertical flow layout for settings pages. Widgets are positioned at build time; the host screen scrolls them. */
public final class PageBuilder {
	public static final int ROW = 20;

	private final int x;
	private final int top;
	private final int width;
	private final Runnable rebuild;
	private final List<AbstractWidget> widgets = new ArrayList<>();
	private int y;

	public PageBuilder(int x, int y, int width, Runnable rebuild) {
		this.x = x;
		this.top = y;
		this.y = y;
		this.width = width;
		this.rebuild = rebuild;
	}

	public int x() {
		return this.x;
	}

	public int y() {
		return this.y;
	}

	public int width() {
		return this.width;
	}

	/** Re-runs the page's build, keeping scroll position and focus (used when a choice reveals more controls). */
	public Runnable rebuild() {
		return this.rebuild;
	}

	public <T extends AbstractWidget> T add(T widget) {
		this.widgets.add(widget);
		return widget;
	}

	public void advance(int height) {
		this.y += height;
	}

	public void space(int height) {
		this.y += height;
	}

	public void heading(Component text) {
		this.label(() -> text, LabelWidget.Tone.HEADING);
		this.y += 1;
	}

	public void label(Component text, LabelWidget.Tone tone) {
		this.label(() -> text, tone);
	}

	public void label(Supplier<Component> text, LabelWidget.Tone tone) {
		LabelWidget label = new LabelWidget(this.x, this.y, this.width, text, tone);
		this.add(label);
		this.y += label.getHeight() + 4;
	}

	/** Evenly split a row into {@code count} cells and let the factory create the widget for each. */
	public void row(int count, int height, int gap, CellFactory factory) {
		int cellW = (this.width - gap * (count - 1)) / count;
		for (int i = 0; i < count; i++) {
			int cx = this.x + i * (cellW + gap);
			int w = i == count - 1 ? this.x + this.width - cx : cellW;
			AbstractWidget widget = factory.create(i, cx, this.y, w, height);
			if (widget != null) {
				this.add(widget);
			}
		}
		this.y += height + 4;
	}

	public int contentHeight() {
		return this.y - this.top;
	}

	public List<AbstractWidget> widgets() {
		return this.widgets;
	}

	@FunctionalInterface
	public interface CellFactory {
		AbstractWidget create(int index, int x, int y, int w, int h);
	}
}
