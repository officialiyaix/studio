package stabled.client.platform;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.network.chat.Component;
import stabled.client.ui.Rect;
//#if MC >= 26.1
import net.minecraft.client.gui.GuiGraphicsExtractor;
//#else
import net.minecraft.client.gui.GuiGraphics;
//#endif
//#if MC >= 1.21.9
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
//#else
import net.minecraft.client.gui.navigation.CommonInputs;
//#endif

/**
 * Base for every Stabled widget. Translates this Minecraft version's render and input callbacks into a small,
 * version-independent set (draw, pressed/dragged/released, activate, navigate) used by the shared widget code.
 */
public abstract class BaseWidget extends AbstractWidget {
	public enum Nav { LEFT, RIGHT, UP, DOWN, HOME, END }

	private Rect clip;

	protected BaseWidget(int x, int y, int w, int h, Component message) {
		super(x, y, w, h, message);
	}

	protected abstract void draw(Gfx g, int mouseX, int mouseY, float partialTick);

	/** Whether the widget reacts to input (labels and previews don't). */
	protected boolean interactive() {
		return true;
	}

	/** Left mouse button pressed over the widget. Default: {@link #activate()}. */
	protected void pressed(double mouseX, double mouseY) {
		this.activate();
	}

	protected void dragged(double mouseX, double mouseY) {
	}

	protected void released(double mouseX, double mouseY) {
	}

	/** Click, Enter or Space. */
	protected void activate() {
	}

	/** Arrow, Home and End keys while focused. Return true when handled. */
	protected boolean navigate(Nav nav) {
		return false;
	}

	/** Limits hover and clicks to a visible region (null for none), for widgets inside scroll areas. */
	public void setClip(Rect clip) {
		this.clip = clip;
	}

//#if MC >= 26.1
	@Override
	protected final void extractWidgetRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float a) {
		this.draw(new Gfx(graphics), mouseX, mouseY, a);
		if (this.interactive()) {
			this.handleCursor(graphics);
		}
	}
//#else
	@Override
	protected final void renderWidget(GuiGraphics graphics, int mouseX, int mouseY, float a) {
		this.draw(new Gfx(graphics), mouseX, mouseY, a);
	//#if MC >= 1.21.11
		if (this.interactive()) {
			this.handleCursor(graphics);
		}
	//#endif
	}
//#endif

	@Override
	public boolean isMouseOver(double mouseX, double mouseY) {
		return this.interactive() && super.isMouseOver(mouseX, mouseY) && (this.clip == null || this.clip.contains(mouseX, mouseY));
	}

	private static Nav nav(int key) {
		return switch (key) {
			case InputConstants.KEY_LEFT -> Nav.LEFT;
			case InputConstants.KEY_RIGHT -> Nav.RIGHT;
			case InputConstants.KEY_UP -> Nav.UP;
			case InputConstants.KEY_DOWN -> Nav.DOWN;
			case InputConstants.KEY_HOME -> Nav.HOME;
			case InputConstants.KEY_END -> Nav.END;
			default -> null;
		};
	}

	private boolean handleKey(int key, boolean selection) {
		if (!this.isActive() || !this.interactive()) {
			return false;
		}
		if (selection) {
			this.playDownSound(Minecraft.getInstance().getSoundManager());
			this.activate();
			return true;
		}
		Nav nav = nav(key);
		return nav != null && this.navigate(nav);
	}

//#if MC >= 1.21.9
	@Override
	public void onClick(MouseButtonEvent event, boolean doubleClick) {
		this.pressed(event.x(), event.y());
	}

	@Override
	protected void onDrag(MouseButtonEvent event, double dx, double dy) {
		this.dragged(event.x(), event.y());
	}

	@Override
	public void onRelease(MouseButtonEvent event) {
		this.released(event.x(), event.y());
	}

	@Override
	public boolean keyPressed(KeyEvent event) {
		return this.handleKey(event.key(), event.isSelection());
	}
//#else
	@Override
	public boolean mouseClicked(double mouseX, double mouseY, int button) {
		// Older versions hit-test clicks without isMouseOver, so apply the clip here as well.
		return this.isMouseOver(mouseX, mouseY) && super.mouseClicked(mouseX, mouseY, button);
	}

	@Override
	public void onClick(double mouseX, double mouseY) {
		this.pressed(mouseX, mouseY);
	}

	@Override
	protected void onDrag(double mouseX, double mouseY, double dx, double dy) {
		this.dragged(mouseX, mouseY);
	}

	@Override
	public void onRelease(double mouseX, double mouseY) {
		this.released(mouseX, mouseY);
	}

	@Override
	public boolean keyPressed(int key, int scanCode, int modifiers) {
		return this.handleKey(key, CommonInputs.selected(key));
	}
//#endif
}
