use serde::{Deserialize, Serialize};
use crate::minecraft_version::MinecraftVersion;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoaderConfig {
	Paper { version: MinecraftVersion},
	Thunderstorm { version: MinecraftVersion},
}