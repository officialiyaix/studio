package stabled.client.ui.screen;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.PlainTextButton;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.contents.TranslatableContents;
import stabled.client.StabledClient;
import stabled.client.discord.PresenceManager;
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
import stabled.client.theme.Theme;
import stabled.client.theme.ThemeManager;
import stabled.client.ui.Ui;

/**
 * Re-arranges the vanilla title screen's own widgets into the Stabled layout: a branded menu column on the left
 * and a small Save card on the right. Every vanilla control (including demo, Realms, friends, language and
 * accessibility) keeps its original behaviour; buttons added by other mods are kept in the column.
 */
public final class TitleLayout {
	private static int panelX;
	private static int panelY;
	private static int panelW;
	private static int panelH;
	private static int logoY;
	private static int logoH;

	private TitleLayout() {
	}

	private static String key(AbstractWidget w) {
		return w.getMessage().getContents() instanceof TranslatableContents t ? t.getKey() : "";
	}

	public static void apply(TitleScreen screen, Consumer<AbstractWidget> add, int width, int height) {
		Minecraft mc = Minecraft.getInstance();
		List<AbstractWidget> widgets = new ArrayList<>(Platform.widgets(screen));
		List<AbstractWidget> play = new ArrayList<>();
		List<AbstractWidget> icons = new ArrayList<>();
		List<AbstractWidget> extra = new ArrayList<>();
		AbstractWidget options = null;
		AbstractWidget quit = null;
		for (String k : new String[] {"menu.singleplayer", "menu.multiplayer", "menu.online", "menu.playdemo", "menu.resetdemo"}) {
			for (AbstractWidget w : widgets) {
				if (k.equals(key(w))) {
					play.add(w);
				}
			}
		}
		for (AbstractWidget w : widgets) {
			String k = key(w);
			if (play.contains(w) || w instanceof PlainTextButton) {
				continue;
			}
			if ("menu.options".equals(k)) {
				options = w;
			} else if ("menu.quit".equals(k)) {
				quit = w;
			} else if (w.getWidth() <= 24 && w.getHeight() <= 24) {
				icons.add(w);
			} else {
				extra.add(w);
			}
		}

		Button stabled = Button.builder(Component.translatable("stabled.menu.settings"),
			b -> Platform.setScreen(new StabledSettingsScreen(screen))).build();
		add.accept(stabled);

		panelW = Math.max(170, Math.min(230, Math.round(width * 0.3F)));
		if (width < 420) {
			panelW = Math.min(width - 16, 230);
		}
		panelX = width >= 520 ? Math.max(16, width / 18) : (width - panelW) / 2;
		int pad = 10;
		int innerW = panelW - pad * 2;
		int rows = play.size() + 1 + extra.size() + 1 + (icons.isEmpty() ? 0 : 1);
		boolean compact = height < 300;
		int gap = compact ? 2 : 4;
		int groupGap = compact ? 4 : 8;
		logoH = compact ? 20 : 28;
		int header = logoH + 14 + groupGap;
		int content = header + rows * (20 + gap) + groupGap * 3;
		panelH = Math.min(height - 16, content + pad * 2);
		panelY = Math.max(8, (height - panelH) / 2);
		logoY = panelY + pad;

		int x = panelX + pad;
		int y = logoY + header;
		for (AbstractWidget w : play) {
			place(w, x, y, innerW);
			y += 20 + gap;
		}
		y += groupGap;
		place(stabled, x, y, innerW);
		y += 20 + gap;
		for (AbstractWidget w : extra) {
			place(w, x, y, innerW);
			y += 20 + gap;
		}
		y += groupGap;
		int half = (innerW - gap) / 2;
		if (options != null) {
			place(options, x, y, quit != null ? half : innerW);
		}
		if (quit != null) {
			place(quit, options != null ? x + innerW - half : x, y, options != null ? half : innerW);
		}
		y += 20 + gap + groupGap;
		int ix = x;
		for (AbstractWidget w : icons) {
			w.setPosition(ix, y);
			ix += w.getWidth() + 4;
		}
	}

	private static void place(AbstractWidget w, int x, int y, int width) {
		w.setPosition(x, y);
		w.setWidth(width);
	}

	/** Background, menu panel and branding, drawn underneath the vanilla widgets. */
	public static void background(Screen screen, Gfx g, float partialTick) {
		int width = screen.width;
		int height = screen.height;
		Ui.menuBackdrop(g, width, height);
		Theme t = Ui.theme();
		if (Ui.appearance().isGlass()) {
			Ui.roundRect(g, panelX, panelY, panelW, panelH, 5, Ui.surface(t.surface()));
		} else {
			Ui.roundRect(g, panelX, panelY, panelW, panelH, 5, t.surface());
		}
		Ui.roundOutline(g, panelX, panelY, panelW, panelH, 5, Ui.outline(t.border()));
		g.fill(panelX + 1, panelY + 1, panelX + panelW - 1, panelY + 2, Ui.alpha(t.accent(), 0.7F));
		Ui.logo(g, panelX + 10, logoY, logoH, 0xFFFFFFFF);
		Ui.textClipped(g, Component.translatable("stabled.menu.subtitle"), panelX + 10, logoY + logoH + 3, panelW - 20, t.textMuted());
		saveCard(g, width, height, t);
	}

	private static void saveCard(Gfx g, int width, int height, Theme t) {
		if (width < 520) {
			return;
		}
		Minecraft mc = Minecraft.getInstance();
		Component save = Component.literal(StabledClient.saveName());
		Component version = Component.translatable("stabled.menu.version", PresenceManager.get().gameVersion());
		Component user = Component.translatable("stabled.menu.signed_in", mc.getUser().getName());
		PresenceManager.Status status = PresenceManager.get().status();
		Component discord = Component.translatable("stabled.menu.discord." + status.name().toLowerCase(java.util.Locale.ROOT));
		int w = Math.min(190, Math.max(Ui.font().width(save), Ui.font().width(user)) + 26);
		w = Math.max(w, 150);
		int h = 52;
		int x = width - w - 12;
		int y = 12;
		Ui.roundRect(g, x, y, w, h, 4, Ui.surface(t.surface()));
		Ui.roundOutline(g, x, y, w, h, 4, Ui.outline(t.border()));
		Ui.mark(g, x + 8, y + 8, 14);
		Ui.textClipped(g, save, x + 27, y + 8, w - 35, t.text());
		Ui.textClipped(g, version, x + 27, y + 18, w - 35, t.textMuted());
		Ui.textClipped(g, user, x + 8, y + 30, w - 16, t.textMuted());
		int dot = status == PresenceManager.Status.CONNECTED ? t.success() : (status == PresenceManager.Status.DISABLED ? t.textMuted() : t.warning());
		Ui.roundRect(g, x + 8, y + 42, 5, 5, 2, dot);
		Ui.textClipped(g, discord, x + 16, y + 41, w - 24, t.textMuted());
	}

}
