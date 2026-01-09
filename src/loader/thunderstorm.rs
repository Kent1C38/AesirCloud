use async_trait::async_trait;
use crate::errors::CloudError;
use crate::loader::LoaderBackend;
use crate::minecraft_version::MinecraftVersion;
use crate::screen_manager::JavaVersion;

pub struct ThunderstormLoader {
	pub version: MinecraftVersion,
}

#[async_trait]
impl LoaderBackend for ThunderstormLoader {
	fn name(&self) -> &'static str {
		"thunderstorm"
	}

	fn version(&self) -> MinecraftVersion {
		self.version.clone()
	}

	fn java_version(&self) -> JavaVersion {
		JavaVersion::J25
	}

	async fn resolve_download_url(&self) -> Result<String, CloudError> {
		Ok("".to_string())
	}
}