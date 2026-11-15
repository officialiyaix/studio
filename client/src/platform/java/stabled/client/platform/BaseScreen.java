package stabled.client.platform;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
//#if MC >= 26.1
import net.minecraft.client.gui.GuiGraphicsExtractor;
//#else
import net.minecraft.client.gui.GuiGraphics;
//#endif

/**
 * Base for Stabled's own screens. The background and foreground layers are drawn through {@link Gfx}, below the
 * widgets registered with addRenderableWidget, whichever way this Minecraft version orders screen rendering.
 */
public abstract class BaseScreen extends Screen {
	private float partialTick;

	protected BaseScreen(Component title) {
		super(title);
	}

	protected abstract void drawBackground(Gfx g, int mouseX, int mouseY, float partialTick);

	/** Drawn above the background and below widgets registered with addRenderableWidget. */
	protected abstract void drawForeground(Gfx g, int mouseX, int mouseY, float partialTick);

	protected boolean scrolled(double mouseX, double mouseY, double amount) {
		return false;
	}

	/**
	 * Vanilla menu blur of whatever is behind the screen (respects the player's blur setting). Minecraft 1.20.4 and
	 * older have no menu blur, so there it draws nothing.
	 */
	protected void menuBlur(Gfx g) {
//#if MC >= 26.1
		this.extractBlurredBackground(g.raw);
//#elif MC >= 1.21.6
		this.renderBlurredBackground(g.raw);
//#elif MC >= 1.21.2
		this.renderBlurredBackground();
//#elif MC >= 1.20.5
		this.renderBlurredBackground(this.partialTick);
//#endif
	}

	/** Draws a widget that is registered for input only (addWidget), e.g. inside a clipped scroll area. */
	protected void drawChild(AbstractWidget widget, Gfx g, int mouseX, int mouseY, float partialTick) {
//#if MC >= 26.1
		widget.extractRenderState(g.raw, mouseX, mouseY, partialTick);
//#else
		widget.render(g.raw, mouseX, mouseY, partialTick);
//#endif
	}

	private void drawLayers(Gfx g, int mouseX, int mouseY, float partialTick) {
		this.partialTick = partialTick;
		this.drawBackground(g, mouseX, mouseY, partialTick);
		this.drawForeground(g, mouseX, mouseY, partialTick);
	}

//#if MC >= 26.1
	@Override
	public void extractBackground(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float a) {
		this.drawLayers(new Gfx(graphics), mouseX, mouseY, a);
	}
//#elif MC >= 1.20.2
	@Override
	public void renderBackground(GuiGraphics graphics, int mouseX, int mouseY, float a) {
		this.drawLayers(new Gfx(graphics), mouseX, mouseY, a);
	}
//#else
	@Override
	public void render(GuiGraphics graphics, int mouseX, int mouseY, float a) {
		this.drawLayers(new Gfx(graphics), mouseX, mouseY, a);
		super.render(graphics, mouseX, mouseY, a);
	}
//#endif

//#if MC >= 1.20.2
	@Override
	public boolean mouseScrolled(double x, double y, double scrollX, double scrollY) {
		return this.scrolled(x, y, scrollY) || super.mouseScrolled(x, y, scrollX, scrollY);
	}
//#else
	@Override
	public boolean mouseScrolled(double x, double y, double amount) {
		return this.scrolled(x, y, amount) || super.mouseScrolled(x, y, amount);
	}
//#endif

//#if MC >= 1.20.5 && MC < 1.21.6
	/** The panorama shared by vanilla screens (a protected member of Screen, hence drawn from here). */
	static void renderSharedPanorama(Gfx g, Minecraft mc) {
	//#if MC >= 1.21.2
		PANORAMA.render(g.raw, g.width(), g.height(), 1F, 0F);
	//#elif MC >= 1.21
		PANORAMA.render(g.raw, g.width(), g.height(), 1F, mc.getTimer().getRealtimeDeltaTicks());
	//#else
		PANORAMA.render(g.raw, g.width(), g.height(), 1F, mc.getDeltaFrameTime());
	//#endif
	}
//#endif
}
