package stabled.client.emote;

import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.model.HumanoidModel;
import net.minecraft.network.chat.Component;
import stabled.client.StabledClient;
import stabled.client.platform.Gfx;
import stabled.client.platform.Platform;
import stabled.client.theme.Theme;
import stabled.client.ui.Ui;

/**
 * Plays an emote on the local player's model. While an emote plays, the camera switches to third person (front)
 * so the player can see it, and switches back when it ends. Moving stops it. Emotes are drawn by this client only:
 * there's no server component, so other players don't see them.
 */
public final class EmotePlayer {
	private static final float FADE_IN = 0.18F;
	private static final float FADE_OUT = 0.25F;
	/** Blocks the player may drift (knockback, ice) before moving counts as stopping the emote. */
	private static final double MOVE_LIMIT = 0.35;

	private static Emote active;
	private static long startedAt;
	private static long stoppingAt;
	private static double startX;
	private static double startZ;
	private static CameraType restoreCamera;
	private static CameraType ourCamera;

	private EmotePlayer() {
	}

	public static Emote active() {
		return active;
	}

	public static void play(Emote emote) {
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null) {
			return;
		}
		if (active == null && StabledClient.settings().emotesThirdPerson && mc.options.getCameraType().isFirstPerson()) {
			restoreCamera = mc.options.getCameraType();
			ourCamera = CameraType.THIRD_PERSON_FRONT;
			mc.options.setCameraType(ourCamera);
		}
		active = emote;
		startedAt = System.nanoTime();
		stoppingAt = 0;
		startX = mc.player.getX();
		startZ = mc.player.getZ();
	}

	/** Fades the current emote out. */
	public static void stop() {
		if (active != null && stoppingAt == 0) {
			stoppingAt = System.nanoTime();
		}
	}

	private static void finish() {
		active = null;
		stoppingAt = 0;
		Minecraft mc = Minecraft.getInstance();
		// Only switch back if the player didn't change the camera themselves in the meantime.
		if (restoreCamera != null && mc.options.getCameraType() == ourCamera) {
			mc.options.setCameraType(restoreCamera);
		}
		restoreCamera = null;
		ourCamera = null;
	}

	private static float seconds(long since) {
		return (System.nanoTime() - since) / 1_000_000_000F;
	}

	public static void tick(Minecraft mc) {
		if (active == null) {
			return;
		}
		if (mc.player == null || mc.level == null) {
			finish();
			return;
		}
		if (stoppingAt != 0) {
			if (seconds(stoppingAt) >= FADE_OUT) {
				finish();
			}
			return;
		}
		double dx = mc.player.getX() - startX;
		double dz = mc.player.getZ() - startZ;
		if (seconds(startedAt) >= active.duration - FADE_OUT || dx * dx + dz * dz > MOVE_LIMIT * MOVE_LIMIT) {
			stop();
		}
	}

	/** Called at the end of the player model's setupAnim. */
	public static void pose(HumanoidModel<?> model, int entityId) {
		Emote emote = active;
		Minecraft mc = Minecraft.getInstance();
		if (emote == null || mc.player == null || entityId != mc.player.getId()) {
			return;
		}
		float t = seconds(startedAt);
		float w = Math.min(1F, t / FADE_IN);
		if (stoppingAt != 0) {
			w = Math.min(w, Math.max(0F, 1F - seconds(stoppingAt) / FADE_OUT));
		}
		w = w * w * (3F - 2F * w);
		emote.pose(new Emote.Parts(model.head, model.body, model.rightArm, model.leftArm, model.rightLeg, model.leftLeg), t, w);
	}

	/** Whether {@link #pose} changes this entity's model (so version code can copy the pose to overlay parts). */
	public static boolean posing(int entityId) {
		Minecraft mc = Minecraft.getInstance();
		return active != null && mc.player != null && entityId == mc.player.getId();
	}

	/** A small pill at the top of the HUD while an emote plays. */
	public static void drawHud(Gfx g) {
		Emote emote = active;
		if (emote == null || Platform.hudHidden() || Platform.screen() != null) {
			return;
		}
		Theme t = Ui.theme();
		Component text = Component.translatable("stabled.emotes.playing", emote.label());
		int w = Ui.font().width(text) + 34;
		int h = 18;
		int x = (g.width() - w) / 2;
		int y = 6;
		float progress = Math.min(1F, seconds(startedAt) / emote.duration);
		Ui.roundRect(g, x, y, w, h, 4, Ui.alpha(Ui.opaque(t.surface()), 0.86F));
		Ui.roundOutline(g, x, y, w, h, 4, Ui.alpha(t.accent(), 0.7F));
		g.text(emote.glyph, x + 8, y + 5, t.accent(), false);
		Ui.text(g, text, x + 22, y + 5, t.text());
		g.fill(x + 4, y + h - 2, x + 4 + Math.round((w - 8) * progress), y + h - 1, t.accent());
	}
}
