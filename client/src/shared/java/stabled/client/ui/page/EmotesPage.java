package stabled.client.ui.page;

import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.emote.Emote;
import stabled.client.emote.EmotePlayer;
import stabled.client.platform.Platform;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.StabledButton;
import stabled.client.ui.widget.ToggleWidget;

/** Emotes: how to open the wheel, the camera option, and a button to try each emote. */
public final class EmotesPage implements Page {
	@Override
	public Component title() {
		return Component.translatable("stabled.page.emotes");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.emotes.summary", StabledClient.emoteKey().getTranslatedKeyMessage());
	}

	@Override
	public void build(PageBuilder b) {
		b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("stabled.emotes.third_person"),
			() -> StabledClient.settings().emotesThirdPerson, v -> StabledClient.update(s -> s.emotesThirdPerson = v)));
		b.advance(PageBuilder.ROW + 4);
		b.label(Component.translatable("stabled.emotes.third_person.desc"), LabelWidget.Tone.MUTED);
		b.label(Component.translatable("stabled.emotes.local"), LabelWidget.Tone.MUTED);
		b.space(4);
		b.heading(Component.translatable("stabled.emotes.try"));
		boolean inWorld = Minecraft.getInstance().player != null;
		if (!inWorld) {
			b.label(Component.translatable("stabled.emotes.need_world"), LabelWidget.Tone.MUTED);
		}
		Emote[] emotes = Emote.values();
		int cols = b.width() >= 300 ? 4 : 2;
		int gap = 4;
		int w = (b.width() - gap * (cols - 1)) / cols;
		for (int i = 0; i < emotes.length; i++) {
			Emote e = emotes[i];
			int x = b.x() + (i % cols) * (w + gap);
			int y = b.y() + (i / cols) * (PageBuilder.ROW + gap);
			StabledButton button = new StabledButton(x, y, w, PageBuilder.ROW, Component.literal((i + 1) + "  " + e.glyph + "  ").append(e.label()),
				StabledButton.Style.SECONDARY, btn -> {
					Platform.setScreen(null);
					EmotePlayer.play(e);
				});
			button.active = inWorld;
			b.add(button);
		}
		b.advance(((emotes.length + cols - 1) / cols) * (PageBuilder.ROW + gap));
	}
}
