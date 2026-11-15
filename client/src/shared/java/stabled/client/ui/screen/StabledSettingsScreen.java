package stabled.client.ui.screen;

import java.util.List;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;
import stabled.client.ui.page.AppearancePage;
import stabled.client.ui.page.DiscordPage;
import stabled.client.ui.page.EmotesPage;
import stabled.client.ui.page.FpsPage;
import stabled.client.ui.page.MenusPage;
import stabled.client.ui.page.MotionBlurPage;
import stabled.client.ui.page.Page;
import stabled.client.ui.page.PerformancePage;
import stabled.client.ui.widget.ChoiceButton;
import stabled.client.ui.widget.StabledButton;

/** Stabled Settings: every in-game Stabled option, persisted immediately to this Save's settings. */
public final class StabledSettingsScreen extends StabledScreen {
	private static int lastPage;
	private final Screen parent;
	private final List<Page> pages;
	private int page;
	private int cardX;
	private int cardY;
	private int cardW;
	private int cardH;

	public StabledSettingsScreen(Screen parent) {
		super(Component.translatable("stabled.settings.title"));
		this.parent = parent;
		this.pages = List.of(new AppearancePage(), new PerformancePage(), new FpsPage(), new MotionBlurPage(), new EmotesPage(), new DiscordPage(false), new MenusPage(this));
		this.page = Math.max(0, Math.min(lastPage, this.pages.size() - 1));
	}

	@Override
	protected void initChrome() {
		this.cardW = Math.min(this.width - 16, 560);
		this.cardH = Math.min(this.height - 16, 360);
		this.cardX = (this.width - this.cardW) / 2;
		this.cardY = (this.height - this.cardH) / 2;
		int header = 30;
		int footer = 30;
		boolean sideNav = this.cardW >= 400;
		int navW = sideNav ? 112 : 0;
		int navRows = sideNav ? 0 : 2;
		if (sideNav) {
			int y = this.cardY + header + 4;
			for (int i = 0; i < this.pages.size(); i++) {
				this.addNav(i, this.cardX + 8, y, navW - 12);
				y += 22;
			}
		} else {
			int perRow = 3;
			int cellW = (this.cardW - 16 - 4 * (perRow - 1)) / perRow;
			for (int i = 0; i < this.pages.size(); i++) {
				int r = i / perRow, c = i % perRow;
				this.addNav(i, this.cardX + 8 + c * (cellW + 4), this.cardY + header + r * 22, cellW);
			}
		}
		this.contentX = this.cardX + navW + (sideNav ? 0 : 4);
		this.contentY = this.cardY + header + navRows * 22 + 2;
		this.contentW = this.cardW - navW - (sideNav ? 6 : 8);
		this.contentH = this.cardH - header - footer - navRows * 22 - 4;
		this.addRenderableWidget(new StabledButton(this.cardX + this.cardW - 88, this.cardY + this.cardH - 26, 80, 20,
			Component.translatable("gui.done"), StabledButton.Style.PRIMARY, b -> this.onClose()));
	}

	private void addNav(int index, int x, int y, int w) {
		this.addRenderableWidget(new ChoiceButton(x, y, w, 20, this.pages.get(index).title(), () -> this.page == index, () -> {
			if (this.page != index) {
				this.page = index;
				lastPage = index;
				this.switchPage();
			}
		}));
	}

	@Override
	protected Page currentPage() {
		return this.pages.get(this.page);
	}

	@Override
	protected void extractChrome(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		Ui.panel(g, this.cardX, this.cardY, this.cardW, this.cardH, 5);
		g.fill(this.cardX + 1, this.cardY + 1, this.cardX + this.cardW - 1, this.cardY + 2, Ui.alpha(t.accent(), 0.8F));
		Ui.mark(g, this.cardX + 8, this.cardY + 8, 14);
		Ui.text(g, this.title.copy().withStyle(s -> s.withBold(true)), this.cardX + 27, this.cardY + 8, t.text());
		Component save = Component.literal(StabledClient.saveName());
		int sw = Ui.font().width(save);
		Ui.textClipped(g, save, this.cardX + this.cardW - Math.min(sw, this.cardW / 2) - 10, this.cardY + 9, this.cardW / 2, t.textMuted());
		g.fill(this.cardX + 1, this.cardY + 26, this.cardX + this.cardW - 1, this.cardY + 27, Ui.outline(t.border()));
		Ui.roundRect(g, this.contentX, this.contentY, this.contentW, this.contentH, 3, Ui.surface(t.surfaceAlt()));
		g.fill(this.cardX + 1, this.cardY + this.cardH - 31, this.cardX + this.cardW - 1, this.cardY + this.cardH - 30, Ui.outline(t.border()));
		Ui.textClipped(g, Component.translatable("stabled.settings.autosave"), this.cardX + 10, this.cardY + this.cardH - 20, this.cardW - 110, t.textMuted());
	}

	@Override
	public void onClose() {
		Platform.setScreen(this.parent);
	}
}
