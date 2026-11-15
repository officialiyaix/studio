package stabled.client.platform;

import net.minecraft.client.KeyMapping;
//#if MC >= 1.21.9
import net.minecraft.client.input.KeyEvent;
//#endif

/** A key press or release delivered to a Stabled screen, the same on every Minecraft version. */
public final class KeyInput {
//#if MC >= 1.21.9
	private final KeyEvent event;

	KeyInput(KeyEvent event) {
		this.event = event;
	}

	public int key() {
		return this.event.key();
	}

	public boolean matches(KeyMapping mapping) {
		return mapping.matches(this.event);
	}
//#else
	private final int key;
	private final int scanCode;

	KeyInput(int key, int scanCode) {
		this.key = key;
		this.scanCode = scanCode;
	}

	public int key() {
		return this.key;
	}

	public boolean matches(KeyMapping mapping) {
		return mapping.matches(this.key, this.scanCode);
	}
//#endif
}
