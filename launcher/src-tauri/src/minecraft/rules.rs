//! Evaluation of Mojang version-JSON rules (libraries and arguments) for the current platform.

use serde_json::Value;

pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn os_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    }
}

/// `rules` absent → allowed. Otherwise the last matching rule decides.
pub fn allowed(rules: Option<&Value>, features: &[(&str, bool)]) -> bool {
    let Some(rules) = rules.and_then(Value::as_array) else { return true };
    let mut result = false;
    for rule in rules {
        let allow = rule.get("action").and_then(Value::as_str) == Some("allow");
        let mut matches = true;
        if let Some(os) = rule.get("os") {
            if let Some(name) = os.get("name").and_then(Value::as_str) {
                matches &= name == os_name();
            }
            if let Some(arch) = os.get("arch").and_then(Value::as_str) {
                matches &= arch == os_arch();
            }
        }
        if let Some(fs) = rule.get("features").and_then(Value::as_object) {
            for (k, v) in fs {
                let want = v.as_bool().unwrap_or(false);
                let have = features.iter().find(|(n, _)| n == k).map(|(_, b)| *b).unwrap_or(false);
                matches &= want == have;
            }
        }
        if matches {
            result = allow;
        }
    }
    result
}

/// Natives classifiers for arm64 Windows/macOS are only useful on those architectures.
pub fn library_applies_to_arch(name: &str) -> bool {
    let is_arm_native = name.ends_with(":natives-windows-arm64") || name.ends_with(":natives-macos-arm64") || name.ends_with(":natives-linux-arm64");
    if is_arm_native {
        return cfg!(target_arch = "aarch64");
    }
    if name.ends_with(":natives-windows") || name.ends_with(":natives-macos") || name.ends_with(":natives-linux") {
        return !cfg!(target_arch = "aarch64") || name.ends_with(":natives-macos");
    }
    true
}

/// Expand `arguments.game` / `arguments.jvm` into plain strings.
pub fn expand_arguments(list: Option<&Value>, features: &[(&str, bool)]) -> Vec<String> {
    let mut out = Vec::new();
    for item in list.and_then(Value::as_array).into_iter().flatten() {
        match item {
            Value::String(s) => out.push(s.clone()),
            Value::Object(_) => {
                if allowed(item.get("rules"), features) {
                    match item.get("value") {
                        Some(Value::String(s)) => out.push(s.clone()),
                        Some(Value::Array(a)) => out.extend(a.iter().filter_map(Value::as_str).map(str::to_owned)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Maven coordinate → relative path, e.g. `org.ow2.asm:asm:9.10.1` → `org/ow2/asm/asm/9.10.1/asm-9.10.1.jar`.
pub fn maven_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next();
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{}/{artifact}/{version}/{file}", group.replace('.', "/")))
}

/// `group:artifact[:classifier]` without the version — used to let Fabric's libraries override vanilla ones.
pub fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.len() {
        0..=2 => name.to_string(),
        3 => format!("{}:{}", parts[0], parts[1]),
        _ => format!("{}:{}:{}", parts[0], parts[1], parts[3]),
    }
}
