package stabled.client.ui.widget;

import java.util.function.Supplier;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** Non-interactive wrapped text: section headings, explanations and status lines. */
public class LabelWidget extends BaseWidget {
	public enum Tone { HEADING, BODY, MUTED, WARNING, SUCCESS, DANGER }

	private final Supplier<Component> text;
	private final Tone tone;

	public LabelWidget(int x, int y, int w, Supplier<Component> text, Tone tone) {
		super(x, y, w, Math.max(9, Ui.wrappedHeight(text.get(), w)), text.get());
		this.text = text;
		this.tone = tone;
		this.active = false;
	}

	@Override
	protected boolean interactive() {
		return false;
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		int color = switch (this.tone) {
			case HEADING, BODY -> t.text();
			case MUTED -> t.textMuted();
			case WARNING -> t.warning();
			case SUCCESS -> t.success();
			case DANGER -> t.danger();
		};
		Component c = this.text.get();
		if (this.tone == Tone.HEADING) {
			c = c.copy().withStyle(s -> s.withBold(true));
		}
		Ui.wrapped(g, c, this.getX(), this.getY(), this.getWidth(), color);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, this.text.get());
	}
}
