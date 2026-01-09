use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[derive(Debug)]
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
}
