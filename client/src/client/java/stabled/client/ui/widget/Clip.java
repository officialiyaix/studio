package stabled.client.ui.widget;

import net.minecraft.client.gui.navigation.ScreenRectangle;
import org.jspecify.annotations.Nullable;

/** Viewport clipping for widgets that live inside a scrolled content area. */
public interface Clip {
	void setClip(@Nullable ScreenRectangle clip);

	static boolean allows(@Nullable ScreenRectangle clip, double x, double y) {
		return clip == null || clip.containsPoint((int) x, (int) y);
	}
}
