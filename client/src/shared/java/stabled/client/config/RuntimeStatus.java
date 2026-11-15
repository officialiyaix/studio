package stabled.client.config;

import com.google.gson.JsonObject;
import java.time.Instant;

/**
 * Live status written by the game for the launcher ({@code <saveDir>/stabled/runtime.json}). It is only written when
 * something changes, and the launcher treats it as stale unless the recorded process id is still running.
 */
public final class RuntimeStatus {
	private RuntimeStatus() {
	}

	public static void write(String discordStatus, String discordDetail, String motionBlurUnavailable, boolean introOpen) {
		JsonObject o = new JsonObject();
		o.addProperty("version", 1);
		o.addProperty("pid", ProcessHandle.current().pid());
		o.addProperty("updatedAt", Instant.now().toString());
		JsonObject discord = new JsonObject();
		discord.addProperty("status", discordStatus);
		if (discordDetail != null) {
			discord.addProperty("detail", discordDetail);
		}
		o.add("discord", discord);
		JsonObject blur = new JsonObject();
		blur.addProperty("available", motionBlurUnavailable == null);
		if (motionBlurUnavailable != null) {
			blur.addProperty("reason", motionBlurUnavailable);
		}
		o.add("motionBlur", blur);
		o.addProperty("introductionOpen", introOpen);
		JsonFiles.writeObject(StabledPaths.runtimeFile(), o);
	}
}
