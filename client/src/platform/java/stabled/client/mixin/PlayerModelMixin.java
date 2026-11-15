package stabled.client.mixin;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import stabled.client.emote.EmotePlayer;
//#if MC >= 1.21.11
import net.minecraft.client.model.player.PlayerModel;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
//#elif MC >= 1.21.9
import net.minecraft.client.model.PlayerModel;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
//#elif MC >= 1.21.2
import net.minecraft.client.model.PlayerModel;
import net.minecraft.client.renderer.entity.state.PlayerRenderState;
//#else
import net.minecraft.client.model.PlayerModel;
import net.minecraft.world.entity.LivingEntity;
//#endif

/**
 * Emotes: after vanilla has posed the player model, the local player's emote (if any) adjusts it. From 1.21.2 the
 * hat, sleeves, jacket and trousers are children of the parts they cover, so they follow automatically; before
 * that they are copied from them, as vanilla does.
 */
@Mixin(PlayerModel.class)
public abstract class PlayerModelMixin {
//#if MC >= 1.21.9
	@Inject(method = "setupAnim(Lnet/minecraft/client/renderer/entity/state/AvatarRenderState;)V", at = @At("TAIL"))
	private void stabled$emote(AvatarRenderState state, CallbackInfo ci) {
		EmotePlayer.pose((PlayerModel) (Object) this, state.id);
	}
//#elif MC >= 1.21.2
	@Inject(method = "setupAnim(Lnet/minecraft/client/renderer/entity/state/PlayerRenderState;)V", at = @At("TAIL"))
	private void stabled$emote(PlayerRenderState state, CallbackInfo ci) {
		EmotePlayer.pose((PlayerModel) (Object) this, state.id);
	}
//#else
	@Inject(method = "setupAnim(Lnet/minecraft/world/entity/LivingEntity;FFFFF)V", at = @At("TAIL"))
	private void stabled$emote(LivingEntity entity, float limbSwing, float limbSwingAmount, float age, float headYaw, float headPitch, CallbackInfo ci) {
		if (!EmotePlayer.posing(entity.getId())) {
			return;
		}
		PlayerModel<?> model = (PlayerModel<?>) (Object) this;
		EmotePlayer.pose(model, entity.getId());
		model.hat.copyFrom(model.head);
		model.leftSleeve.copyFrom(model.leftArm);
		model.rightSleeve.copyFrom(model.rightArm);
		model.leftPants.copyFrom(model.leftLeg);
		model.rightPants.copyFrom(model.rightLeg);
		model.jacket.copyFrom(model.body);
	}
//#endif
}
