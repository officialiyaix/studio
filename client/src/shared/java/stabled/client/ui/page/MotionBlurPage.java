package stabled.client.ui.page;

import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.config.ClientSettings;
import stabled.client.platform.MotionBlur;
import stabled.client.ui.widget.LabelWidget;
import stabled.client.ui.widget.SliderWidget;
import stabled.client.ui.widget.ToggleWidget;

/** Step 4 / Motion Blur: toggle, strength and its performance cost. */
public final class MotionBlurPage implements Page {
	@Override
	public Component title() {
		return Component.translatable("stabled.page.motion_blur");
	}

	@Override
	public Component summary() {
		return Component.translatable("stabled.page.motion_blur.summary");
	}

	@Override
	public void build(PageBuilder b) {
		String unavailable = MotionBlur.unavailableReason();
		ToggleWidget toggle = b.add(new ToggleWidget(b.x(), b.y(), b.width(), PageBuilder.ROW, Component.translatable("stabled.motion_blur.toggle"),
			() -> StabledClient.settings().motionBlurEnabled, v -> {
				if (v) {
					MotionBlur.resetFailure();
				}
				StabledClient.update(s -> s.motionBlurEnabled = v);
				b.rebuild().run();
			}));
		b.advance(PageBuilder.ROW + 6);
		SliderWidget strength = b.add(SliderWidget.percentOfRange(b.x(), b.y(), b.width(), 22, Component.translatable("stabled.motion_blur.strength"),
			ClientSettings.MIN_BLUR, ClientSettings.MAX_BLUR, StabledClient.settings().motionBlurStrength,
			v -> StabledClient.settings().motionBlurStrength = v,
			v -> StabledClient.update(s -> s.motionBlurStrength = v)));
		strength.active = StabledClient.settings().motionBlurEnabled && unavailable == null;
		b.advance(26);
		if (unavailable != null) {
			toggle.active = StabledClient.settings().motionBlurEnabled; // still allow switching it off
			b.label(Component.literal(unavailable), LabelWidget.Tone.WARNING);
		}
		b.heading(Component.translatable("stabled.motion_blur.cost_heading"));
		b.label(Component.translatable("stabled.motion_blur.cost"), LabelWidget.Tone.MUTED);
		b.label(Component.translatable("stabled.motion_blur.where"), LabelWidget.Tone.MUTED);
	}
}
