package stabled.client.ui.widget;

import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;

/** A non-interactive area drawn by a callback (previews, sample panels). */
public class DrawWidget extends BaseWidget {
	@FunctionalInterface
	public interface Painter {
		void paint(Gfx g, int x, int y, int w, int h, float partialTick);
	}

	private final Painter painter;

	public DrawWidget(int x, int y, int w, int h, Component narration, Painter painter) {
		super(x, y, w, h, narration);
		this.painter = painter;
		this.active = false;
	}

	@Override
	protected boolean interactive() {
		return false;
	}

	@Override
	protected void draw(Gfx g, int mouseX, int mouseY, float a) {
		this.painter.paint(g, this.getX(), this.getY(), this.getWidth(), this.getHeight(), a);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, this.getMessage());
	}
}
