package stabled.client.platform;

import com.mojang.blaze3d.platform.InputConstants;
import com.mojang.blaze3d.platform.NativeImage;
import java.util.List;
import java.util.function.BooleanSupplier;
import java.util.function.Consumer;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.network.chat.Component;
import net.minecraft.world.effect.MobEffectInstance;
import net.fabricmc.fabric.api.client.screen.v1.Screens;
//#if MC >= 1.21.5
import com.mojang.blaze3d.systems.RenderSystem;
//#endif
//#if MC >= 26.2
import java.util.Arrays;
import net.minecraft.client.PreferredGraphicsApi;
//#endif
//#if MC >= 26.1
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
//#else
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
//#endif
//#if MC >= 1.21.6
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
//#else
import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;
//#endif
//#if MC >= 1.21
import net.minecraft.client.gui.screens.options.OptionsScreen;
//#else
import net.minecraft.client.gui.screens.OptionsScreen;
//#endif
//#if MC >= 1.20.2
import net.minecraft.client.multiplayer.ServerData;
//#endif

/** Version-specific hooks used by the shared client code; one implementation per Minecraft version range. */
public final class Platform {
//#if MC >= 1.21.9
	private static final KeyMapping.Category CATEGORY = KeyMapping.Category.register(Gfx.id("stabled:main"));
//#else
	private static final String CATEGORY = "key.category.stabled.main";
//#endif

	private Platform() {
	}

	private static Minecraft mc() {
		return Minecraft.getInstance();
	}

	public static void registerHud(Consumer<Gfx> draw) {
//#if MC >= 1.21.6
		HudElementRegistry.addLast(Gfx.id("stabled:fps"), (graphics, delta) -> draw.accept(new Gfx(graphics)));
//#else
		HudRenderCallback.EVENT.register((graphics, delta) -> draw.accept(new Gfx(graphics)));
//#endif
	}

	/** Registers a key binding (default Right Shift) and returns its "was pressed" check. */
	public static BooleanSupplier registerKey(String translationKey) {
		KeyMapping mapping = new KeyMapping(translationKey, InputConstants.KEY_RSHIFT, CATEGORY);
//#if MC >= 26.1
		KeyMappingHelper.registerKeyMapping(mapping);
//#else
		KeyBindingHelper.registerKeyBinding(mapping);
//#endif
		return mapping::consumeClick;
	}

	public static Screen screen() {
//#if MC >= 26.2
		return mc().gui.screen();
//#else
		return mc().screen;
//#endif
	}

	public static void setScreen(Screen screen) {
//#if MC >= 26.2
		mc().gui.setScreen(screen);
//#else
		mc().setScreen(screen);
//#endif
	}

	/** The widgets a screen has registered (buttons and the like). */
	public static List<AbstractWidget> widgets(Screen screen) {
//#if MC >= 26.1
		return Screens.getWidgets(screen);
//#else
		return Screens.getButtons(screen);
//#endif
	}

	public static boolean hudHidden() {
//#if MC >= 26.2
		return mc().gui.hud.isHidden();
//#else
		return mc().options.hideGui;
//#endif
	}

	public static boolean debugOverlayVisible() {
//#if MC >= 1.21.11
		return mc().debugEntries.isOverlayVisible();
//#elif MC >= 1.21.9
		return mc().debugEntries.isF3Visible();
//#elif MC >= 1.20.2
		return mc().getDebugOverlay().showDebugScreen();
//#else
		return mc().options.renderDebug;
//#endif
	}

	public static int fps() {
		return mc().getFps();
	}

	/** Height taken by status-effect icons in the top-right corner (0, 26 or 52). */
	public static int effectIconsHeight() {
		if (mc().player == null) {
			return 0;
		}
		boolean beneficial = false;
		boolean harmful = false;
		for (MobEffectInstance effect : mc().player.getActiveEffects()) {
			if (!effect.showIcon()) {
				continue;
			}
//#if MC >= 1.20.5
			boolean good = effect.getEffect().value().isBeneficial();
//#else
			boolean good = effect.getEffect().isBeneficial();
//#endif
			if (good) {
				beneficial = true;
			} else {
				harmful = true;
			}
		}
		return (beneficial ? 26 : 0) + (harmful ? 26 : 0);
	}

	public static boolean playingOnRealms() {
//#if MC >= 1.20.2
		ServerData server = mc().getCurrentServer();
		return server != null && server.isRealm();
//#else
		return mc().isConnectedToRealms();
//#endif
	}

	public static void openVanillaOptions(Screen parent) {
//#if MC >= 26.1 && MC < 26.3
		setScreen(new OptionsScreen(parent, mc().options, mc().level != null));
//#else
		setScreen(new OptionsScreen(parent, mc().options));
//#endif
	}

	public static String rendererName() {
		try {
//#if MC >= 26.2
			return RenderSystem.getDevice().getDeviceInfo().backendName();
//#elif MC >= 1.21.5
			return RenderSystem.getDevice().getBackendName();
//#else
			return "OpenGL";
//#endif
		} catch (RuntimeException e) {
			return "?";
		}
	}

	/** Graphics APIs the player can choose between (empty on versions with only OpenGL). */
	public static List<String> graphicsApis() {
//#if MC >= 26.2
		return Arrays.stream(PreferredGraphicsApi.values()).map(PreferredGraphicsApi::getSerializedName).toList();
//#else
		return List.of();
//#endif
	}

	public static Component graphicsApiLabel(String id) {
//#if MC >= 26.2
		return api(id).caption();
//#else
		return Component.literal("OpenGL");
//#endif
	}

	public static String graphicsApi() {
//#if MC >= 26.2
		return mc().options.preferredGraphicsBackend().get().getSerializedName();
//#else
		return "opengl";
//#endif
	}

	public static void setGraphicsApi(String id) {
//#if MC >= 26.2
		mc().options.preferredGraphicsBackend().set(api(id));
		mc().options.save();
//#endif
	}

//#if MC >= 26.2
	private static PreferredGraphicsApi api(String id) {
		for (PreferredGraphicsApi a : PreferredGraphicsApi.values()) {
			if (a.getSerializedName().equals(id)) {
				return a;
			}
		}
		return PreferredGraphicsApi.DEFAULT;
	}
//#endif

	public static void applyPreset(String preset) {
		Presets.apply(preset);
	}

	/** Registers (or replaces) a dynamic texture; the image is owned by the texture afterwards. */
	public static void loadTexture(String id, NativeImage image) {
//#if MC >= 1.21.5
		DynamicTexture texture = new DynamicTexture(() -> "Stabled " + id, image);
//#else
		DynamicTexture texture = new DynamicTexture(image);
//#endif
		mc().getTextureManager().register(Gfx.id(id), texture);
	}

	public static void releaseTexture(String id) {
		mc().getTextureManager().release(Gfx.id(id));
	}
}
