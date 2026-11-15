package stabled.client.ui.screen;

import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.events.GuiEventListener;
import net.minecraft.network.chat.Component;
import stabled.client.platform.BaseScreen;
import stabled.client.platform.BaseWidget;
import stabled.client.platform.Gfx;
import stabled.client.theme.Theme;
import stabled.client.ui.Rect;
import stabled.client.ui.Ui;
import stabled.client.ui.page.Page;
import stabled.client.ui.page.PageBuilder;

/**
 * Base for Stabled's own screens: themed backdrop, and a scrollable content area that hosts a {@link Page}.
 * Content widgets are clipped to the viewport for both drawing and mouse input; keyboard focus scrolls the
 * focused control into view, so every control stays reachable at any GUI scale.
 */
public abstract class StabledScreen extends BaseScreen {
	private static final int SCROLL_STEP = 14;

	protected int contentX;
	protected int contentY;
	protected int contentW;
	protected int contentH;
	private final List<AbstractWidget> content = new ArrayList<>();
	private final Map<AbstractWidget, Integer> baseY = new IdentityHashMap<>();
	private int scroll;
	private int contentHeight;
	private int restoreFocusIndex = -1;
	private boolean keepScroll;

	protected StabledScreen(Component title) {
		super(title);
	}

	/** Lays out the screen chrome (header, footer, navigation) and sets the content rectangle. */
	protected abstract void initChrome();

	protected abstract Page currentPage();

	@Override
	protected final void init() {
		int savedScroll = this.scroll;
		this.content.clear();
		this.baseY.clear();
		this.initChrome();
		Page page = this.currentPage();
		if (page != null) {
			int pad = 8;
			PageBuilder b = new PageBuilder(this.contentX + pad, this.contentY + pad, this.contentW - pad * 2 - 4, this::rebuildPage);
			b.label(page.summary(), stabled.client.ui.widget.LabelWidget.Tone.MUTED);
			b.space(2);
			page.build(b);
			this.contentHeight = b.contentHeight() + pad * 2;
			Rect clip = new Rect(this.contentX, this.contentY, this.contentW, this.contentH);
			for (AbstractWidget w : b.widgets()) {
				this.content.add(w);
				this.baseY.put(w, w.getY());
				if (w instanceof BaseWidget c) {
					c.setClip(clip);
				}
				this.addWidget(w);
			}
		}
		this.scroll = this.keepScroll ? savedScroll : 0;
		this.keepScroll = false;
		this.applyScroll();
		if (this.restoreFocusIndex >= 0 && this.restoreFocusIndex < this.content.size()) {
			AbstractWidget w = this.content.get(this.restoreFocusIndex);
			if (w.active) {
				this.setFocused(w);
			}
		}
		this.restoreFocusIndex = -1;
	}

	/** Rebuilds only after the current input event finishes, so a widget never rebuilds itself mid-click. */
	protected void rebuildPage() {
		GuiEventListener focused = this.getFocused();
		this.restoreFocusIndex = focused instanceof AbstractWidget w ? this.content.indexOf(w) : -1;
		this.keepScroll = true;
		this.minecraft.execute(this::rebuildWidgets);
	}

	protected void switchPage() {
		this.scroll = 0;
		this.rebuildWidgets();
	}

	private int maxScroll() {
		return Math.max(0, this.contentHeight - this.contentH);
	}

	private void applyScroll() {
		this.scroll = Math.max(0, Math.min(this.scroll, this.maxScroll()));
		for (AbstractWidget w : this.content) {
			w.setY(this.baseY.get(w) - this.scroll);
		}
	}

	@Override
	protected boolean scrolled(double x, double y, double amount) {
		if (x >= this.contentX && x < this.contentX + this.contentW && y >= this.contentY && y < this.contentY + this.contentH && this.maxScroll() > 0) {
			this.scroll -= (int) Math.round(amount * SCROLL_STEP);
			this.applyScroll();
			return true;
		}
		return false;
	}

	@Override
	public void setFocused(GuiEventListener listener) {
		super.setFocused(listener);
		if (listener instanceof AbstractWidget w && this.baseY.containsKey(w)) {
			int top = w.getY();
			int bottom = w.getY() + w.getHeight();
			if (top < this.contentY + 4) {
				this.scroll -= (this.contentY + 4) - top;
				this.applyScroll();
			} else if (bottom > this.contentY + this.contentH - 4) {
				this.scroll += bottom - (this.contentY + this.contentH - 4);
				this.applyScroll();
			}
		}
	}

	@Override
	public void tick() {
		Page page = this.currentPage();
		if (page != null) {
			page.tick();
		}
	}

	@Override
	protected void drawBackground(Gfx g, int mouseX, int mouseY, float a) {
		if (this.minecraft.level == null) {
			Ui.menuBackdrop(g, this.width, this.height);
		} else {
			if (Ui.appearance().isGlass()) {
				this.menuBlur(g);
			}
			Ui.worldScrim(g, this.width, this.height);
		}
	}

	@Override
	protected void drawForeground(Gfx g, int mouseX, int mouseY, float a) {
		this.extractChrome(g, mouseX, mouseY, a);
		g.scissor(this.contentX, this.contentY, this.contentX + this.contentW, this.contentY + this.contentH);
		boolean inside = mouseX >= this.contentX && mouseX < this.contentX + this.contentW && mouseY >= this.contentY && mouseY < this.contentY + this.contentH;
		int mx = inside ? mouseX : -10_000;
		int my = inside ? mouseY : -10_000;
		for (AbstractWidget w : this.content) {
			if (w.getY() + w.getHeight() >= this.contentY && w.getY() <= this.contentY + this.contentH) {
				this.drawChild(w, g, mx, my, a);
			}
		}
		g.endScissor();
		this.extractScrollbar(g);
	}

	/** Panels and text drawn behind the widgets. */
	protected abstract void extractChrome(Gfx g, int mouseX, int mouseY, float a);

	private void extractScrollbar(Gfx g) {
		int max = this.maxScroll();
		if (max <= 0) {
			return;
		}
		Theme t = Ui.theme();
		int trackX = this.contentX + this.contentW - 4;
		int trackY = this.contentY + 2;
		int trackH = this.contentH - 4;
		int thumbH = Math.max(14, trackH * this.contentH / Math.max(1, this.contentHeight));
		int thumbY = trackY + (trackH - thumbH) * this.scroll / max;
		g.fill(trackX, trackY, trackX + 2, trackY + trackH, Ui.alpha(t.border(), 0.6F));
		g.fill(trackX, thumbY, trackX + 2, thumbY + thumbH, t.accent());
	}
}
