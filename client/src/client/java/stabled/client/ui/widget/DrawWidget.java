package stabled.client.ui.widget;

import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;

/** A non-interactive area drawn by a callback (previews, sample panels). */
public class DrawWidget extends AbstractWidget {
	@FunctionalInterface
	public interface Painter {
		void paint(GuiGraphicsExtractor g, int x, int y, int w, int h, float partialTick);
	}

	private final Painter painter;

	public DrawWidget(int x, int y, int w, int h, Component narration, Painter painter) {
		super(x, y, w, h, narration);
		this.painter = painter;
		this.active = false;
	}

	@Override
	protected void extractWidgetRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float a) {
		this.painter.paint(g, this.getX(), this.getY(), this.getWidth(), this.getHeight(), a);
	}

	@Override
	public boolean isMouseOver(double mouseX, double mouseY) {
		return false;
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, this.getMessage());
	}
}
