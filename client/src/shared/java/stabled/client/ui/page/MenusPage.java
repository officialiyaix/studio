package stabled.client.ui.page;

import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.config.StabledPaths;
import stabled.client.platform.Platform;
import stabled.client.ui.screen.IntroScreen;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.StabledButton;
import stabled.client.ui.widget.ToggleWidget;

/** Settings-only page: menu styling, replaying the introduction and quick access to vanilla options. */
public final class MenusPage implements Page {
	private final Screen owner;

	public MenusPage(Screen owner) {
		this.owner = owner;
	}

	@Override
	public Component title() {
		return Component.translatable("stabled.page.menus");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.menus.summary");
	}

	@Override
	public void build(PageBuilder b) {
		b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("stabled.menus.styled"),
			() -> StabledClient.settings().styledMenus, v -> StabledClient.update(s -> s.styledMenus = v)));
		b.advance(PageBuilder.ROW + 4);
		b.label(Component.translatable("stabled.menus.styled.desc"), LabelWidget.Tone.MUTED);
		b.row(2, PageBuilder.ROW, 4, (i, x, y, w, h) -> i == 0
			? new StabledButton(x, y, w, h, Component.translatable("stabled.menus.replay"), StabledButton.Style.SECONDARY, btn -> {
				StabledClient.onboarding().reset();
				Platform.setScreen(new IntroScreen(this.owner));
			})
			: new StabledButton(x, y, w, h, Component.translatable("menu.options"), StabledButton.Style.SECONDARY,
				btn -> Platform.openVanillaOptions(this.owner)));
		b.label(Component.translatable("stabled.menus.replay.desc"), LabelWidget.Tone.MUTED);
		b.space(4);
		b.heading(Component.translatable("stabled.menus.about"));
		b.label(Component.translatable(StabledPaths.launchedByStabled() ? "stabled.menus.launched" : "stabled.menus.dev"), LabelWidget.Tone.MUTED);
	}
}
