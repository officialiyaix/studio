package stabled.client.platform;

import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import net.minecraft.util.FormattedCharSequence;
//#if MC >= 26.1
import net.minecraft.client.gui.GuiGraphicsExtractor;
//#else
import net.minecraft.client.gui.GuiGraphics;
//#endif
//#if MC >= 1.21.11
import net.minecraft.resources.Identifier;
//#else
import net.minecraft.resources.ResourceLocation;
//#endif
//#if MC >= 1.21.6
import net.minecraft.client.renderer.RenderPipelines;
//#elif MC >= 1.21.2
import net.minecraft.client.renderer.RenderType;
//#else
import com.mojang.blaze3d.systems.RenderSystem;
//#endif
//#if MC < 1.20.5
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.renderer.PanoramaRenderer;
//#endif

/**
 * Drawing facade used by Stabled's shared UI code. It wraps this Minecraft version's GUI drawing class
 * (GuiGraphics up to 1.21.11, GuiGraphicsExtractor from 26.1) behind one small, stable set of methods.
 */
public final class Gfx {
//#if MC >= 1.21.11
	private static final Map<String, Identifier> IDS = new HashMap<>();
//#else
	private static final Map<String, ResourceLocation> IDS = new HashMap<>();
//#endif
//#if MC < 1.20.5
	private static PanoramaRenderer panorama;
//#endif

//#if MC >= 26.1
	final GuiGraphicsExtractor raw;

	public Gfx(GuiGraphicsExtractor raw) {
		this.raw = raw;
	}
//#else
	final GuiGraphics raw;

	public Gfx(GuiGraphics raw) {
		this.raw = raw;
	}
//#endif

//#if MC >= 1.21.11
	static Identifier id(String id) {
		return IDS.computeIfAbsent(id, Identifier::parse);
	}
//#elif MC >= 1.21
	static ResourceLocation id(String id) {
		return IDS.computeIfAbsent(id, ResourceLocation::parse);
	}
//#else
	static ResourceLocation id(String id) {
		return IDS.computeIfAbsent(id, ResourceLocation::new);
	}
//#endif

	public int width() {
		return this.raw.guiWidth();
	}

	public int height() {
		return this.raw.guiHeight();
	}

	public void fill(int x0, int y0, int x1, int y1, int argb) {
		this.raw.fill(x0, y0, x1, y1, argb);
	}

	public void gradient(int x0, int y0, int x1, int y1, int top, int bottom) {
		this.raw.fillGradient(x0, y0, x1, y1, top, bottom);
	}

	public void text(Component text, int x, int y, int argb, boolean shadow) {
//#if MC >= 26.1
		this.raw.text(Minecraft.getInstance().font, text, x, y, argb, shadow);
//#else
		this.raw.drawString(Minecraft.getInstance().font, text, x, y, argb, shadow);
//#endif
	}

	public void text(FormattedCharSequence text, int x, int y, int argb, boolean shadow) {
//#if MC >= 26.1
		this.raw.text(Minecraft.getInstance().font, text, x, y, argb, shadow);
//#else
		this.raw.drawString(Minecraft.getInstance().font, text, x, y, argb, shadow);
//#endif
	}

	public void text(String text, int x, int y, int argb, boolean shadow) {
//#if MC >= 26.1
		this.raw.text(Minecraft.getInstance().font, text, x, y, argb, shadow);
//#else
		this.raw.drawString(Minecraft.getInstance().font, text, x, y, argb, shadow);
//#endif
	}

	/** Draws the region (u, v, srcW, srcH) of a texture stretched to (x, y, w, h), multiplied by {@code argb}. */
	public void texture(String texture, int x, int y, int w, int h, float u, float v, int srcW, int srcH, int texW, int texH, int argb) {
//#if MC >= 1.21.6
		this.raw.blit(RenderPipelines.GUI_TEXTURED, id(texture), x, y, u, v, w, h, srcW, srcH, texW, texH, argb);
//#elif MC >= 1.21.2
		this.raw.blit(RenderType::guiTextured, id(texture), x, y, u, v, w, h, srcW, srcH, texW, texH, argb);
//#else
		RenderSystem.enableBlend();
		this.raw.setColor(((argb >> 16) & 0xFF) / 255F, ((argb >> 8) & 0xFF) / 255F, (argb & 0xFF) / 255F, (argb >>> 24) / 255F);
		this.raw.blit(id(texture), x, y, w, h, u, v, srcW, srcH, texW, texH);
		this.raw.setColor(1F, 1F, 1F, 1F);
		RenderSystem.disableBlend();
//#endif
	}

	public void scissor(int x0, int y0, int x1, int y1) {
		this.raw.enableScissor(x0, y0, x1, y1);
	}

	public void endScissor() {
		this.raw.disableScissor();
	}

	public void push() {
//#if MC >= 1.21.6
		this.raw.pose().pushMatrix();
//#else
		this.raw.pose().pushPose();
//#endif
	}

	public void translate(float x, float y) {
//#if MC >= 1.21.6
		this.raw.pose().translate(x, y);
//#else
		this.raw.pose().translate(x, y, 0F);
//#endif
	}

	public void scale(float x, float y) {
//#if MC >= 1.21.6
		this.raw.pose().scale(x, y);
//#else
		this.raw.pose().scale(x, y, 1F);
//#endif
	}

	public void pop() {
//#if MC >= 1.21.6
		this.raw.pose().popMatrix();
//#else
		this.raw.pose().popPose();
//#endif
	}

	/** The vanilla rotating title-screen panorama, covering the whole screen. */
	public void panorama(float partialTick) {
		Minecraft mc = Minecraft.getInstance();
//#if MC >= 26.2
		mc.gameRenderer.panorama().extractRenderState(this.raw, this.raw.guiWidth(), this.raw.guiHeight());
//#elif MC >= 26.1
		mc.gameRenderer.getPanorama().extractRenderState(this.raw, this.raw.guiWidth(), this.raw.guiHeight(), true);
//#elif MC >= 1.21.6
		mc.gameRenderer.getPanorama().render(this.raw, this.raw.guiWidth(), this.raw.guiHeight(), true);
//#elif MC >= 1.20.5
		BaseScreen.renderSharedPanorama(this, mc);
//#else
		if (panorama == null) {
			panorama = new PanoramaRenderer(TitleScreen.CUBE_MAP);
		}
		panorama.render(mc.getDeltaFrameTime(), 1F);
//#endif
	}
}
