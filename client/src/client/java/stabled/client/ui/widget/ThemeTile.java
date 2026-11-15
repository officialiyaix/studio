package stabled.client.ui.widget;

import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.narration.NarratedElementType;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.client.gui.navigation.ScreenRectangle;
import net.minecraft.client.input.InputWithModifiers;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;
import stabled.client.theme.Theme;
import stabled.client.theme.ThemeManager;
import stabled.client.ui.Ui;

/** A selectable theme preview tile (one of the nine choices). */
public class ThemeTile extends AbstractButton implements Clip {
	private final String themeId;
	private final Runnable onSelect;
	private @Nullable ScreenRectangle clip;

	public ThemeTile(int x, int y, int w, int h, String themeId, Runnable onSelect) {
		super(x, y, w, h, Component.literal(ThemeManager.CUSTOM_ID.equals(themeId) ? ThemeManager.customName() : ThemeManager.resolve(themeId).name()));
		this.themeId = themeId;
		this.onSelect = onSelect;
	}

	public boolean isSelected() {
		return this.themeId.equals(ThemeManager.appearance().theme);
	}

	@Override
	public void onPress(InputWithModifiers input) {
		this.onSelect.run();
	}

	@Override
	protected void extractContents(GuiGraphicsExtractor g, int mouseX, int mouseY, float a) {
		Theme active = Ui.theme();
		Theme preview = ThemeManager.resolve(this.themeId);
		boolean custom = ThemeManager.CUSTOM_ID.equals(this.themeId);
		boolean sel = this.isSelected();
		boolean hot = this.isHoveredOrFocused();
		int x = this.getX(), y = this.getY(), w = this.getWidth(), h = this.getHeight();
		int swatchH = Math.max(10, h - 12);

		// Swatch: a miniature launcher window in the tile's own colours.
		Identifier bgTex = custom ? ThemeManager.customBackgroundTexture() : null;
		if (bgTex != null) {
			g.enableScissor(x + 1, y + 1, x + w - 1, y + swatchH);
			Ui.coverBlit(g, bgTex, x + 1, y + 1, w - 2, swatchH - 1, ThemeManager.backgroundWidth(), ThemeManager.backgroundHeight());
			g.disableScissor();
			g.fill(x + 1, y + 1, x + w - 1, y + swatchH, Ui.alpha(0xFF000000, (float) ThemeManager.customDim() * 0.6F));
		} else {
			g.fillGradient(x + 1, y + 1, x + w - 1, y + swatchH, preview.bg(), preview.bgAlt());
		}
		if (custom && bgTex == null) {
			Ui.centered(g, Component.translatable("stabled.theme.no_image"), x + w / 2, y + swatchH / 2 - 4, active.textMuted());
		} else {
			int px = x + 5, py = y + 4, pw = Math.max(8, w / 2 - 4), ph = Math.max(4, swatchH - 8);
			Ui.roundRect(g, px, py, pw, ph, 2, Ui.alpha(preview.surface(), custom ? 0.75F : 1F));
			g.fill(px + 3, py + 3, px + pw - 3, py + 5, preview.accent());
			g.fill(px + 3, py + 7, px + Math.max(4, pw / 2), py + 8, preview.textMuted());
			int bx = x + w - 5 - Math.max(8, w / 4);
			Ui.roundRect(g, bx, y + swatchH - 9, Math.max(8, w / 4), 5, 2, preview.cta());
			Ui.roundRect(g, x + w / 2 + 2, py + 1, 6, 4, 2, preview.accentAlt());
		}

		// Label row.
		int labelColor = sel ? active.text() : active.textMuted();
		Ui.textClipped(g, this.getMessage(), x + 4, y + h - 10, w - (sel ? 16 : 8), labelColor);
		if (sel) {
			Ui.check(g, x + w - 11, y + h - 9, active.accent());
		}

		int outline = sel ? active.accent() : (hot ? Ui.mix(active.border(), active.accent(), 0.5F) : active.border());
		Ui.roundOutline(g, x, y, w, h, 3, outline);
		if (sel) {
			Ui.roundOutline(g, x + 1, y + 1, w - 2, h - 2, 2, Ui.alpha(active.accent(), 0.55F));
		}
		if (this.isFocused()) {
			Ui.roundOutline(g, x - 1, y - 1, w + 2, h + 2, 4, Ui.alpha(active.accent(), 0.9F));
		}
		this.handleCursor(g);
	}

	@Override
	protected void updateWidgetNarration(NarrationElementOutput output) {
		output.add(NarratedElementType.TITLE, Component.translatable(this.isSelected() ? "stabled.narration.choice_selected" : "stabled.narration.choice", this.getMessage()));
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
