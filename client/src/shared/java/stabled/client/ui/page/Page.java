package stabled.client.ui.page;

import net.minecraft.network.chat.Component;

/** One section of Stabled Settings, reused as a step of the first-launch introduction. */
public interface Page {
	Component title();

	/** One short explanation shown above the controls. */
	Component summary();

	void build(PageBuilder b);

	default void tick() {
	}
}
