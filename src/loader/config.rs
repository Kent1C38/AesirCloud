use crate::minecraft_version::MinecraftVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoaderConfig {
    Paper { version: MinecraftVersion },
    Yggdrasil { version: MinecraftVersion },
}

