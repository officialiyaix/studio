package stabled.client.ui.page;

import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Options;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.config.ClientSettings;
import stabled.client.platform.Platform;
import stabled.client.ui.widget.ChoiceButton;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.SliderWidget;
import stabled.client.ui.widget.ToggleWidget;

/** Step 2 / Performance: Stabled preset plus the few settings that matter most, applied immediately. */
public final class PerformancePage implements Page {
	private static final String RUNNING_BACKEND = Platform.rendererName();

	static Component label(String preset) {
		return Component.translatable("stabled.preset." + preset);
	}

	static Component description(String preset) {
		return Component.translatable("stabled.preset." + preset + ".desc");
	}

	@Override
	public Component title() {
		return Component.translatable("stabled.page.performance");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.performance.summary");
	}

	@Override
	public void build(PageBuilder b) {
		Options o = Minecraft.getInstance().options;
		b.heading(Component.translatable("stabled.preset.heading"));
		b.row(ClientSettings.PRESETS.size(), PageBuilder.ROW, 4, (i, x, y, w, h) -> {
			String preset = ClientSettings.PRESETS.get(i);
			return new ChoiceButton(x, y, w, h, label(preset),
				() -> preset.equals(StabledClient.settings().performancePreset), () -> {
					StabledClient.update(s -> {
						s.performancePreset = preset;
						s.performanceApplied = preset;
					});
					Platform.applyPreset(preset);
					b.rebuild().run();
				});
		});
		b.label(description(StabledClient.settings().performancePreset), LabelWidget.Tone.MUTED);

		b.add(new SliderWidget(b.x(), b.y(), b.width(), 22, Component.translatable("options.renderDistance"), 2, 32, 1,
			o.renderDistance().get(), v -> Component.translatable("options.chunks", v), v -> {
			}, v -> {
				if (o.renderDistance().get() != v) {
					o.renderDistance().set(v);
					o.save();
					StabledClient.update(s -> {
						s.performancePreset = "custom";
						s.performanceApplied = "custom";
					});
					b.rebuild().run();
				}
			}));
		b.advance(26);
		int fps = o.framerateLimit().get();
		b.add(new SliderWidget(b.x(), b.y(), b.width(), 22, Component.translatable("options.framerateLimit"), 10, 260, 10, fps,
			v -> v >= 260 ? Component.translatable("options.framerateLimit.max") : Component.translatable("options.framerate", v), v -> {
			}, v -> {
				o.framerateLimit().set(v);
				o.save();
			}));
		b.advance(26);
		b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("options.vsync"),
			() -> o.enableVsync().get(), v -> {
				o.enableVsync().set(v);
				o.save();
			}));
		b.advance(PageBuilder.ROW + 6);

		List<String> apis = Platform.graphicsApis();
		if (!apis.isEmpty()) {
			b.heading(Component.translatable("stabled.graphics_api.heading"));
			b.row(apis.size(), PageBuilder.ROW, 4, (i, x, y, w, h) -> new ChoiceButton(x, y, w, h, Platform.graphicsApiLabel(apis.get(i)),
				() -> apis.get(i).equals(Platform.graphicsApi()), () -> {
					Platform.setGraphicsApi(apis.get(i));
					b.rebuild().run();
				}));
			b.label(Component.translatable("stabled.graphics_api.running", RUNNING_BACKEND), LabelWidget.Tone.MUTED);
			b.label(Component.translatable("stabled.graphics_api.restart"), LabelWidget.Tone.WARNING);
		} else {
			b.label(Component.translatable("stabled.performance.immediate"), LabelWidget.Tone.MUTED);
		}
		b.label(Component.translatable("stabled.performance.mods"), LabelWidget.Tone.MUTED);
	}
}
