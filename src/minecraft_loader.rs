use crate::errors::CloudError;
use crate::file_downloader::download_file;
use crate::minecraft_version::MinecraftVersion;
use crate::screen_manager::JavaVersion;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::path::Path;

#[derive(Deserialize)]
struct DownloadInfo {
    url: String,
}

#[derive(Deserialize)]
struct Build {
    channel: String,
    downloads: std::collections::HashMap<String, DownloadInfo>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "version", rename_all = "lowercase")]
pub enum MinecraftLoader {
    Paper(MinecraftVersion),
    ThunderStorm(MinecraftVersion),
}

impl MinecraftLoader {
    pub fn name(&self) -> &'static str {
        match self {
            MinecraftLoader::Paper(_) => "paper",
            MinecraftLoader::ThunderStorm(_) => "thunderstorm",
        }
    }

    pub fn version(&self) -> &'static str {
        match self {
            MinecraftLoader::Paper(ver) => ver.get(),
            MinecraftLoader::ThunderStorm(ver) => ver.get(),
        }
    }
    pub async fn latest_build_url(&self) -> Result<String, CloudError> {
        let version = self.version();
        let url = format!(
            "https://fill.papermc.io/v3/projects/paper/versions/{}/builds",
            version
        );

        let builds: Vec<Build> = reqwest::Client::new()
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

        for build in builds {
            if build.channel.to_uppercase() == "STABLE" {
                if let Some(info) = build.downloads.get("server:default") {
                    return Ok(info.url.clone());
                }
            }
        }

        Err(CloudError::NoStableBuild)
    }

    pub async fn install(&self) -> Result<(), CloudError> {
        let local = "versions/paper";
        let jar_name = format!("paper-{}.jar", self.version());
        if !Path::new(&local).exists() {
            create_dir_all(&local).map_err(|_| CloudError::FileError)?;
        }

        let url = self.latest_build_url().await?;

        download_file(&url, &format!("{}/{}", local, jar_name)).await?;

        Ok(())
    }
    pub fn is_installed(&self) -> bool {
        let exec = format!("versions/paper/paper-{}.jar", self.version());
        Path::new(&exec).exists()
    }

    pub fn get_java_version(&self) -> JavaVersion {
        match self {
            MinecraftLoader::Paper(ver) => ver.java_version(),
            MinecraftLoader::ThunderStorm(_) => JavaVersion::J25,
        }
    }
}
