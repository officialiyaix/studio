package stabled.client.ui.page;

import java.time.Instant;
import java.util.Locale;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.discord.PresenceManager;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;
import stabled.client.ui.widget.DrawWidget;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.ToggleWidget;

/** Step 5 / Discord Presence: toggle, activity preview and live connection status. */
public final class DiscordPage implements Page {
	private final boolean finalStep;

	public DiscordPage(boolean finalStep) {
		this.finalStep = finalStep;
	}

	@Override
	public Component title() {
		return Component.translatable("stabled.page.discord");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.discord.summary");
	}

	@Override
	public void build(PageBuilder b) {
		b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("stabled.discord.toggle"),
			() -> StabledClient.settings().discordEnabled, v -> StabledClient.update(s -> s.discordEnabled = v)));
		b.advance(PageBuilder.ROW + 4);
		b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("stabled.discord.elapsed"),
			() -> StabledClient.settings().discordShowElapsed, v -> StabledClient.update(s -> s.discordShowElapsed = v)));
		b.advance(PageBuilder.ROW + 6);

		b.label(() -> statusText(), LabelWidget.Tone.BODY);
		int h = 48;
		b.add(new DrawWidget(b.x(), b.y(), Math.min(b.width(), 240), h, Component.translatable("stabled.discord.preview"), (g, x, y, w, ph, pt) -> {
			Theme t = Ui.theme();
			Ui.roundRect(g, x, y, w, ph, 4, 0xFF232428);
			Ui.roundOutline(g, x, y, w, ph, 4, 0xFF3F4147);
			Ui.mark(g, x + 8, y + 8, 32);
			boolean on = StabledClient.settings().discordEnabled;
			PresenceManager pm = PresenceManager.get();
			Ui.text(g, Component.literal("Stabled Client"), x + 48, y + 8, 0xFFF2F3F5);
			if (on) {
				Ui.textClipped(g, Component.literal(pm.activity().details), x + 48, y + 19, w - 56, 0xFFDBDEE1);
				Ui.textClipped(g, Component.literal("Minecraft " + pm.gameVersion()), x + 48, y + 29, w - 56, 0xFFB5BAC1);
				if (StabledClient.settings().discordShowElapsed) {
					long secs = Math.max(0, Instant.now().getEpochSecond() - pm.sessionStartSeconds());
					Ui.text(g, Component.translatable("stabled.discord.elapsed_value", String.format(Locale.ROOT, "%02d:%02d", secs / 60, secs % 60)), x + 48, y + 39 - 1, 0xFF57F287);
				}
			} else {
				Ui.text(g, Component.translatable("stabled.discord.preview_off"), x + 48, y + 22, t.textMuted());
			}
		}));
		b.advance(h + 4);
		b.label(Component.translatable("stabled.discord.preview_label"), LabelWidget.Tone.MUTED);
		b.label(Component.translatable("stabled.discord.privacy"), LabelWidget.Tone.MUTED);
		if (this.finalStep) {
			b.space(2);
			b.label(Component.translatable("stabled.intro.done"), LabelWidget.Tone.SUCCESS);
		}
	}

	static Component statusText() {
		PresenceManager pm = PresenceManager.get();
		return switch (pm.status()) {
			case CONNECTED -> Component.translatable("stabled.discord.status.connected");
			case CONNECTING -> Component.translatable("stabled.discord.status.connecting");
			case WAITING_FOR_DISCORD -> Component.translatable("stabled.discord.status.waiting", pm.secondsUntilRetry());
			case REJECTED -> Component.translatable("stabled.discord.status.rejected");
			case NOT_CONFIGURED -> Component.translatable("stabled.discord.status.not_configured");
			case DISABLED -> Component.translatable("stabled.discord.status.disabled");
		};
	}
}
