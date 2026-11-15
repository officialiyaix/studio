package stabled.client.config;

import com.google.gson.JsonObject;
import java.time.Instant;

/**
 * Per-Save introduction state ({@code <saveDir>/stabled/onboarding.json}).
 * Completion is recorded only by Finish or Skip; the current step is saved as the player moves so that closing
 * the game midway resumes where they left off.
 */
public final class Onboarding {
	public static final int STEPS = 5;

	private JsonObject raw = new JsonObject();
	public boolean completed;
	public boolean skipped;
	public int step = 1;
	public String completedAt;

	public static Onboarding load() {
		JsonObject o = JsonFiles.readObject(StabledPaths.onboardingFile());
		Onboarding s = new Onboarding();
		s.raw = o;
		s.completed = JsonFiles.getBool(o, "completed", false);
		s.skipped = JsonFiles.getBool(o, "skipped", false);
		s.step = Appearance.clamp((int) JsonFiles.getNumber(o, "step", 1), 1, STEPS);
		s.completedAt = JsonFiles.getString(o, "completedAt", null);
		return s;
	}

	public void saveProgress(int step) {
		this.step = Appearance.clamp(step, 1, STEPS);
		this.write();
	}

	public void complete(boolean skipped) {
		this.completed = true;
		this.skipped = skipped;
		this.completedAt = Instant.now().toString();
		this.write();
	}

	public void reset() {
		this.completed = false;
		this.skipped = false;
		this.step = 1;
		this.completedAt = null;
		this.write();
	}

	private void write() {
		JsonObject o = this.raw;
		o.addProperty("version", 1);
		o.addProperty("completed", this.completed);
		o.addProperty("skipped", this.skipped);
		o.addProperty("step", this.step);
		if (this.completedAt != null) {
			o.addProperty("completedAt", this.completedAt);
		} else {
			o.remove("completedAt");
		}
		JsonFiles.writeObject(StabledPaths.onboardingFile(), o);
	}
}
