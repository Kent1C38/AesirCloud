use async_trait::async_trait;
use crate::errors::CloudError;
use crate::loader::LoaderBackend;
use crate::minecraft_version::MinecraftVersion;
use crate::screen_manager::JavaVersion;

pub struct PaperLoader {
	pub version: MinecraftVersion,
}

#[async_trait]
impl LoaderBackend for PaperLoader {
	fn name(&self) -> &'static str {
		"paper"
	}

	fn version(&self) -> MinecraftVersion {
		self.version.clone()
	}

	fn java_version(&self) -> JavaVersion {
		match self.version {
			MinecraftVersion::V1_21_10 |
			MinecraftVersion::V1_21_11
			=> JavaVersion::J21
		}
	}

	async fn resolve_download_url(&self) -> Result<String, CloudError> {
		let url = format!(
			"https://api.papermc.io/v2/projects/paper/versions/{}/builds",
			self.version.get()
		);

		let resp: serde_json::Value = reqwest::get(url).await.map_err(|_| CloudError::HTTPError)?.json().await.map_err(|_| CloudError::JSONError)?;

		let build = resp["builds"]
			.as_array()
			.and_then(|b| b.last())
			.ok_or(CloudError::NoStableBuild)?;

		let build_number = build["build"].as_i64().ok_or(CloudError::JSONError)?;

		Ok(format!(
			"https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/paper-{}-{}.jar",
			self.version.get(), build_number, self.version.get(), build_number
		))
	}
}