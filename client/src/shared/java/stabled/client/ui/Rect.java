package stabled.client.ui;

/** A screen rectangle in GUI pixels (version-independent; vanilla's ScreenRectangle API varies by version). */
public record Rect(int x, int y, int width, int height) {
	public boolean contains(double px, double py) {
		return px >= this.x && px < this.x + this.width && py >= this.y && py < this.y + this.height;
	}
}
