//! Supported Minecraft versions and the pinned Stabled stack for each (stack/<version>.json).
//! Only combinations listed here are offered for new Saves; everything else is shown as unsupported.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const PRIMARY_VERSION: &str = "26.3";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackMod {
    pub mod_id: String,
    pub title: String,
    pub purpose: String,
    pub required: bool,
    pub project: String,
    pub version_id: String,
    pub version_number: String,
    pub version_type: String,
    pub filename: String,
    pub url: String,
    pub sha512: String,
    pub size: u64,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loader {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaSpec {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub minecraft: String,
    pub revision: u32,
    pub loader: Loader,
    pub java: JavaSpec,
    pub mods: Vec<StackMod>,
    /// Why a usual component is missing for this version (e.g. no compatible build exists).
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Stack {
    pub fn key(&self) -> String {
        format!("{}@{}", self.minecraft, self.revision)
    }

    pub fn fabric_profile_id(&self) -> String {
        format!("fabric-loader-{}-{}", self.loader.version, self.minecraft)
    }
}

#[derive(Deserialize)]
struct StacksFile {
    stacks: Vec<Stack>,
}

static STACKS: OnceLock<Vec<Stack>> = OnceLock::new();

/// Every supported version, oldest first (generated and verified by tools/update_stacks.py).
pub fn all() -> &'static Vec<Stack> {
    STACKS.get_or_init(|| serde_json::from_str::<StacksFile>(include_str!("../../stack/stacks.json")).expect("stack/stacks.json").stacks)
}

pub fn for_version(version: &str) -> Option<&'static Stack> {
    all().iter().find(|s| s.minecraft == version)
}

pub fn is_supported(version: &str) -> bool {
    for_version(version).is_some()
}

/// Mod id of the bundled Stabled client. It is required for every Save and protected from removal.
pub const STABLED_MOD_ID: &str = "stabled";
/// File name of the Stabled client inside a Save's mods folder.
pub const STABLED_JAR_NAME: &str = "stabled-client.jar";

/// File name of the bundled Stabled client build for a Minecraft version (resources/clients/).
pub fn bundled_jar_name(minecraft: &str) -> String {
    format!("stabled-client-{minecraft}.jar")
}
