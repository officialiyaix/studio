package stabled.client.ui.screen;

import java.util.List;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
import stabled.client.config.Onboarding;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;
import stabled.client.ui.page.AppearancePage;
import stabled.client.ui.page.DiscordPage;
import stabled.client.ui.page.FpsPage;
import stabled.client.ui.page.MotionBlurPage;
import stabled.client.ui.page.Page;
import stabled.client.ui.page.PerformancePage;
import stabled.client.ui.widget.StabledButton;

/**
 * The five-step introduction shown on a Save's first launch. Every control saves immediately, so moving between
 * steps keeps choices. Completion is recorded only by Finish or Skip; the current step is saved on each move.
 */
public final class IntroScreen extends StabledScreen {
	private final Screen parent;
	private final List<Page> steps = List.of(new AppearancePage(), new PerformancePage(), new FpsPage(), new MotionBlurPage(), new DiscordPage(true));
	private int step;
	private int cardX;
	private int cardY;
	private int cardW;
	private int cardH;

	public IntroScreen(Screen parent) {
		super(Component.translatable("stabled.intro.title"));
		this.parent = parent;
		this.step = StabledClient.onboarding().step - 1;
		StabledClient.setIntroOpen(true);
	}

	@Override
	protected void initChrome() {
		this.cardW = Math.min(this.width - 16, 460);
		this.cardH = Math.min(this.height - 16, 340);
		this.cardX = (this.width - this.cardW) / 2;
		this.cardY = (this.height - this.cardH) / 2;
		int header = 44;
		int footer = 32;
		this.contentX = this.cardX + 6;
		this.contentY = this.cardY + header;
		this.contentW = this.cardW - 12;
		this.contentH = this.cardH - header - footer;

		boolean last = this.step == Onboarding.STEPS - 1;
		int by = this.cardY + this.cardH - 26;
		int bw = Math.min(84, (this.cardW - 32) / 3);
		StabledButton next = new StabledButton(this.cardX + this.cardW - bw - 8, by, bw, 20,
			Component.translatable(last ? "stabled.intro.finish" : "stabled.intro.next"), StabledButton.Style.PRIMARY, b -> this.next());
		StabledButton skip = new StabledButton(next.getX() - bw - 4, by, bw, 20,
			Component.translatable("stabled.intro.skip"), StabledButton.Style.SECONDARY, b -> this.finish(true));
		if (this.step > 0) {
			this.addRenderableWidget(new StabledButton(this.cardX + 8, by, bw, 20,
				Component.translatable("stabled.intro.back"), StabledButton.Style.GHOST, b -> this.back()));
		}
		this.addRenderableWidget(skip);
		this.addRenderableWidget(next);
		this.setInitialFocus(next);
	}

	@Override
	protected Page currentPage() {
		return this.steps.get(this.step);
	}

	private void next() {
		if (this.step >= Onboarding.STEPS - 1) {
			this.finish(false);
			return;
		}
		this.step++;
		StabledClient.onboarding().saveProgress(this.step + 1);
		this.switchPage();
	}

	private void back() {
		if (this.step > 0) {
			this.step--;
			StabledClient.onboarding().saveProgress(this.step + 1);
			this.switchPage();
		}
	}

	private void finish(boolean skipped) {
		// Choices already made were saved as they were made; untouched settings keep this Save's stored values,
		// which are the documented defaults for a new Save.
		StabledClient.onboarding().complete(skipped);
		StabledClient.setIntroOpen(false);
		Platform.setScreen(this.parent);
	}

	@Override
	public boolean shouldCloseOnEsc() {
		return false;
	}

	@Override
	protected void extractChrome(Gfx g, int mouseX, int mouseY, float a) {
		Theme t = Ui.theme();
		Ui.panel(g, this.cardX, this.cardY, this.cardW, this.cardH, 5);
		g.fill(this.cardX + 1, this.cardY + 1, this.cardX + this.cardW - 1, this.cardY + 2, Ui.alpha(t.accent(), 0.8F));
		Ui.mark(g, this.cardX + 8, this.cardY + 7, 14);
		Ui.text(g, this.title.copy().withStyle(s -> s.withBold(true)), this.cardX + 27, this.cardY + 8, t.text());
		Component progress = Component.translatable("stabled.intro.progress", this.step + 1, Onboarding.STEPS);
		int pw = Ui.font().width(progress);
		Ui.text(g, progress, this.cardX + this.cardW - pw - 10, this.cardY + 8, t.textMuted());
		int barX = this.cardX + 8, barW = this.cardW - 16, segGap = 3;
		int segW = (barW - segGap * (Onboarding.STEPS - 1)) / Onboarding.STEPS;
		for (int i = 0; i < Onboarding.STEPS; i++) {
			int color = i <= this.step ? t.accent() : Ui.mix(t.surfaceAlt(), t.border(), 0.7F);
			Ui.roundRect(g, barX + i * (segW + segGap), this.cardY + 22, segW, 3, 1, color);
		}
		Ui.text(g, this.steps.get(this.step).title().copy().withStyle(s -> s.withBold(true)), this.cardX + 10, this.cardY + 31, t.text());
		g.fill(this.cardX + 1, this.cardY + this.cardH - 33, this.cardX + this.cardW - 1, this.cardY + this.cardH - 32, Ui.outline(t.border()));
	}
}
