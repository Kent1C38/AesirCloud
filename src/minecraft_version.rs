use crate::screen_manager::JavaVersion;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub enum MinecraftVersion {
    #[serde(rename = "1.21.10")]
    V1_21_10,
    #[serde(rename = "1.21.11")]
    V1_21_11,
}

impl MinecraftVersion {
    pub fn get(&self) -> &'static str {
        match self {
            MinecraftVersion::V1_21_11 => "1.21.11",
            MinecraftVersion::V1_21_10 => "1.21.10",
        }
    }

    pub fn java_version(&self) -> JavaVersion {
        match self {
            MinecraftVersion::V1_21_10 | MinecraftVersion::V1_21_11 => JavaVersion::J21,
        }
    }
}
