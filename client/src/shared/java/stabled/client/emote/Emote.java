package stabled.client.emote;

import net.minecraft.client.model.geom.ModelPart;
import net.minecraft.network.chat.Component;

/**
 * The emotes on the wheel. Each one poses the player model for a few seconds; angles are in radians, in the model's
 * own space. An arm's xRot of -π points it straight up and -π/2 straight forward. zRot swings an arm sideways:
 * for a hanging right arm positive is outwards, but once the arm is raised above the shoulder positive swings it
 * inwards (the rotation is applied after xRot), so raised arms use the opposite sign. Mirror for the left arm.
 */
public enum Emote {
	WAVE("wave", "✋", 2.8F) {
		@Override
		void pose(Parts p, float t, float w) {
			set(p.rightArm, -2.6F, 0F, -0.75F - 0.3F * sin(t * 9F), w);
			set(p.head, p.head.xRot, p.head.yRot, 0.08F * sin(t * 4.5F), w);
		}
	},
	CLAP("clap", "✦", 2.6F) {
		@Override
		void pose(Parts p, float t, float w) {
			float c = 0.5F + 0.5F * sin(t * 15F);
			set(p.rightArm, -1.25F, -(0.18F + 0.38F * c), 0F, w);
			set(p.leftArm, -1.25F, 0.18F + 0.38F * c, 0F, w);
		}
	},
	DANCE("dance", "♫", 4.2F) {
		@Override
		void pose(Parts p, float t, float w) {
			float b = t * 6.5F;
			set(p.rightArm, -2.5F + 0.45F * sin(b), 0F, -0.75F, w);
			set(p.leftArm, -2.5F - 0.45F * sin(b), 0F, 0.75F, w);
			set(p.body, 0F, 0.28F * sin(b * 0.5F), 0F, w);
			set(p.head, p.head.xRot, p.head.yRot, 0.16F * sin(b), w);
			set(p.rightLeg, 0.35F * sin(b), 0F, 0.05F, w);
			set(p.leftLeg, -0.35F * sin(b), 0F, -0.05F, w);
		}
	},
	POINT("point", "➜", 2.4F) {
		@Override
		void pose(Parts p, float t, float w) {
			set(p.rightArm, -1.57F + p.head.xRot, p.head.yRot, 0F, w);
		}
	},
	CHEER("cheer", "★", 2.6F) {
		@Override
		void pose(Parts p, float t, float w) {
			float j = sin(t * 12F);
			set(p.rightArm, -2.8F + 0.15F * j, 0F, -0.6F - 0.12F * j, w);
			set(p.leftArm, -2.8F + 0.15F * j, 0F, 0.6F + 0.12F * j, w);
			set(p.head, -0.25F, p.head.yRot, 0F, w);
		}
	},
	FACEPALM("facepalm", "☹", 2.8F) {
		@Override
		void pose(Parts p, float t, float w) {
			// Raised forward and swung inwards so the hand covers the face, head bowed into it.
			set(p.rightArm, -2.35F, 0F, 0.55F, w);
			set(p.head, 0.45F + 0.05F * sin(t * 3F), p.head.yRot * 0.3F, 0F, w);
		}
	},
	BOW("bow", "❀", 2.4F) {
		@Override
		void pose(Parts p, float t, float w) {
			float c = sin(Math.min(1F, t / 2.4F) * (float) Math.PI) * w;
			p.body.xRot = 0.55F * c;
			p.head.xRot = p.head.xRot * (1F - c) + 0.45F * c;
			p.body.y += 3.2F * c;
			p.head.y += 4.2F * c;
			p.rightArm.y += 3.2F * c;
			p.leftArm.y += 3.2F * c;
			p.rightArm.xRot = p.rightArm.xRot * (1F - c) + 0.3F * c;
			p.leftArm.xRot = p.leftArm.xRot * (1F - c) + 0.3F * c;
			p.rightLeg.z += 4F * c;
			p.leftLeg.z += 4F * c;
			p.rightLeg.y += 0.2F * c;
			p.leftLeg.y += 0.2F * c;
		}
	},
	SHRUG("shrug", "ツ", 2.2F) {
		@Override
		void pose(Parts p, float t, float w) {
			float c = sin(Math.min(1F, t / 2.2F) * (float) Math.PI);
			set(p.rightArm, -0.35F * c, 0F, 0.6F * c, w);
			set(p.leftArm, -0.35F * c, 0F, -0.6F * c, w);
			set(p.head, p.head.xRot, p.head.yRot, 0.2F * c, w);
		}
	};

	public final String id;
	public final String glyph;
	/** Length in seconds. */
	public final float duration;

	Emote(String id, String glyph, float duration) {
		this.id = id;
		this.glyph = glyph;
		this.duration = duration;
	}

	public Component label() {
		return Component.translatable("stabled.emote." + this.id);
	}

	/** Poses the model at {@code t} seconds into the emote, blended with the vanilla pose by {@code w} (0–1). */
	abstract void pose(Parts p, float t, float w);

	/** The model parts an emote moves. */
	public record Parts(ModelPart head, ModelPart body, ModelPart rightArm, ModelPart leftArm, ModelPart rightLeg, ModelPart leftLeg) {
	}

	static float sin(float x) {
		return (float) Math.sin(x);
	}

	/** Blends a part's rotation towards the target. */
	static void set(ModelPart part, float x, float y, float z, float w) {
		part.xRot += (x - part.xRot) * w;
		part.yRot += (y - part.yRot) * w;
		part.zRot += (z - part.zRot) * w;
	}
}
