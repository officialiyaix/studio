package stabled.client.ui.widget;

import java.util.function.BooleanSupplier;
import java.util.function.Consumer;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/** A labelled on/off switch. The label sits on the left, the switch on the right. */
public class ToggleWidget extends BaseWidget {
	private final BooleanSupplier value;
	private final Consumer<Boolean> onChange;

	public ToggleWidget(int x, int y, int w, int h, Component label, BooleanSupplier value, Consumer<Boolean> onChange) {
		super(x, y, w, h, label);
		this.value = value;
		this.onChange = onChange;
	}

	@Override
	protected void activate() {
		this.onChange.accept(!this.value.getAsBoolean());
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		boolean on = this.value.getAsBoolean();
		boolean hot = this.isHoveredOrFocused() && this.active;
		int x = this.getX(), y = this.getY(), w = this.getWidth(), h = this.getHeight();
		if (hot) {
			Ui.roundRect(g, x, y, w, h, 3, Ui.surface(t.surfaceHover()));
		}
		if (this.isFocused()) {
			Ui.roundOutline(g, x, y, w, h, 3, Ui.alpha(t.accent(), 0.9F));
		}
		int trackW = 22, trackH = 12;
		int tx = x + w - trackW - 4;
		int ty = y + (h - trackH) / 2;
		int track = on ? t.accent() : Ui.mix(t.surfaceAlt(), t.border(), 0.6F);
		if (!this.active) {
			track = Ui.alpha(track, 0.5F);
		}
		Ui.roundRect(g, tx, ty, trackW, trackH, 6, track);
		Ui.roundOutline(g, tx, ty, trackW, trackH, 6, on ? Ui.mix(t.accent(), 0xFFFFFFFF, 0.2F) : t.border());
		int knob = trackH - 4;
		int kx = on ? tx + trackW - knob - 2 : tx + 2;
		Ui.roundRect(g, kx, ty + 2, knob, knob, 4, on ? t.accentText() : t.textMuted());
		Ui.textClipped(g, this.getMessage(), x + 4, y + (h - 8) / 2, w - trackW - 14, this.active ? t.text() : t.textMuted());
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.translatable("stabled.narration.toggle", this.getMessage(),
			this.value.getAsBoolean() ? CommonComponents.OPTION_ON : CommonComponents.OPTION_OFF));
	}
}
