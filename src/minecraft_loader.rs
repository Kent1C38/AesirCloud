use crate::errors::CloudError;
use crate::minecraft_version::MinecraftVersion;
use crate::screen_manager::JavaVersion;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "version", rename_all = "lowercase")]
pub enum MinecraftLoader {
    Paper(MinecraftVersion),
    ThunderStorm(MinecraftVersion),
}

impl MinecraftLoader {
    pub fn version(&self) -> &'static str {
        match self {
            MinecraftLoader::Paper(ver) => ver.get(),
            MinecraftLoader::ThunderStorm(ver) => ver.get(),
        }
    }
    pub async fn latest_build(&self) -> Result<i64, CloudError> {
        let version = self.version();
        let url = format!("https://papermc.io/api/v1/paper/{}/", version);

        let json: serde_json::Value = reqwest::Client::new()
            .get(&url)
            .header(
                "User-Agent",
                "AesirCloud/InDev0.1 (corsiusquentin@gmail.com)",
            )
            .send()
            .await
            .map_err(|e| CloudError::HTTPError)?
            .json()
            .await
            .map_err(|e| CloudError::JSONError)?;

        json.get("builds")
            .and_then(|b| b.get("latest"))
            .and_then(|v| v.as_i64())
            .ok_or(CloudError::JSONError)
    }

    pub async fn download_url(&self) -> Result<String, CloudError> {
        let latest_build = self.latest_build().await?;
        let url = format!(
            "https://papermc.io/api/v1/paper/{}/{}/download",
            self.version(),
            latest_build
        );
        Ok(url)
    }

    pub fn get_java_version(&self) -> JavaVersion {
        match self {
            MinecraftLoader::Paper(ver) => ver.java_version(),
            MinecraftLoader::ThunderStorm(_) => JavaVersion::J25,
        }
    }
}
