pub mod paper;
pub mod thunderstorm;
pub mod config;

use std::fs::create_dir_all;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use crate::errors::CloudError;
use crate::file_downloader::download_file;
use crate::loader::config::LoaderConfig;
use crate::loader::paper::PaperLoader;
use crate::loader::thunderstorm::ThunderstormLoader;
use crate::minecraft_version::MinecraftVersion;
use crate::screen_manager::JavaVersion;

#[async_trait]
pub trait LoaderBackend: Send + Sync {
	fn name(&self) -> &'static str;
	fn version(&self) -> MinecraftVersion;
	fn java_version(&self) -> JavaVersion;

	async fn resolve_download_url(&self) -> Result<String, CloudError>;

	async fn install(&self) -> Result<(), CloudError> {
		let local = format!("versions/{}", self.name());
		let jar_name = format!("{}-{}.jar", self.name(), self.version().get());
		if !Path::new(&local).exists() {
			create_dir_all(&local).map_err(|_| CloudError::FileError)?;
		}

		let url = self.resolve_download_url().await?;

		download_file(&url, &format!("{}/{}", local, jar_name)).await?;

		Ok(())
	}
	fn is_installed(&self) -> bool {
		let exec = format!("versions/{}/{}-{}.jar", self.name(), self.name(), self.version().get());
		Path::new(&exec).exists()
	}
}

pub fn build_loader(
	config: &LoaderConfig
) -> Arc<dyn LoaderBackend> {
	match config {
		LoaderConfig::Paper { version } => {
			Arc::new(PaperLoader {
				version: version.clone()
			})
		}
		LoaderConfig::Thunderstorm { version } => {
			Arc::new(ThunderstormLoader {
				version: version.clone()
			})
		}
	}
}