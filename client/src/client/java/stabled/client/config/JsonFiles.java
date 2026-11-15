package stabled.client.config;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** Small JSON helpers. Writes are atomic so the launcher never reads a half-written file. */
public final class JsonFiles {
	public static final Gson GSON = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create();
	private static final Logger LOGGER = LoggerFactory.getLogger("Stabled");

	private JsonFiles() {
	}

	public static JsonObject readObject(Path file) {
		try {
			if (!Files.isRegularFile(file)) {
				return new JsonObject();
			}
			JsonElement element = JsonParser.parseString(Files.readString(file, StandardCharsets.UTF_8));
			return element.isJsonObject() ? element.getAsJsonObject() : new JsonObject();
		} catch (Exception e) {
			LOGGER.warn("Stabled: could not read {} ({}); using defaults", file.getFileName(), e.getMessage());
			return new JsonObject();
		}
	}

	public static boolean writeObject(Path file, JsonObject object) {
		try {
			Files.createDirectories(file.getParent());
			Path tmp = file.resolveSibling(file.getFileName() + ".tmp");
			Files.writeString(tmp, GSON.toJson(object), StandardCharsets.UTF_8);
			try {
				Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
			} catch (AtomicMoveNotSupportedException e) {
				Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING);
			}
			return true;
		} catch (IOException e) {
			LOGGER.error("Stabled: could not save {}", file, e);
			return false;
		}
	}

	public static long modifiedTime(Path file) {
		try {
			return Files.isRegularFile(file) ? Files.getLastModifiedTime(file).toMillis() : 0L;
		} catch (IOException e) {
			return 0L;
		}
	}

	static String getString(JsonObject o, String key, String fallback) {
		JsonElement e = o.get(key);
		return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isString() ? e.getAsString() : fallback;
	}

	static boolean getBool(JsonObject o, String key, boolean fallback) {
		JsonElement e = o.get(key);
		return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isBoolean() ? e.getAsBoolean() : fallback;
	}

	static double getNumber(JsonObject o, String key, double fallback) {
		JsonElement e = o.get(key);
		return e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber() ? e.getAsDouble() : fallback;
	}

	static JsonObject getObject(JsonObject o, String key) {
		JsonElement e = o.get(key);
		if (e != null && e.isJsonObject()) {
			return e.getAsJsonObject();
		}
		JsonObject created = new JsonObject();
		o.add(key, created);
		return created;
	}
}
