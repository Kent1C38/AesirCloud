use crate::errors::CloudError;
use crate::file_downloader::download_file;
use flate2::read::GzDecoder;
use std::fs::{File, create_dir_all};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tar::Archive;
use tokio::fs::remove_file;
use tokio::sync::Mutex;
use crate::instance::Instance;
use crate::loader::build_loader;

pub enum JavaVersion {
    J21,
    J25,
}

impl JavaVersion {
    pub fn download_url(&self) -> &'static str {
        match self {
            JavaVersion::J21 => {
                "https://download.oracle.com/java/21/latest/jdk-21_linux-x64_bin.tar.gz"
            }
            JavaVersion::J25 => {
                "https://download.oracle.com/java/25/latest/jdk-25_linux-x64_bin.tar.gz"
            }
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            JavaVersion::J21 => "jdk21",
            JavaVersion::J25 => "jdk25",
        }
    }

    pub fn local_path(&self) -> String {
        format!(".jdk/{}", self.folder_name())
    }

    pub async fn install(&self) -> Result<(), CloudError> {
        let local = self.local_path();
        let folder = self.folder_name();
        if !Path::new(&local).exists() {
            create_dir_all(&local).map_err(|_| CloudError::FileError)?;
        }

        let url = self.download_url();
        let archive_path = format!("{}/{}.tar.gz", local, folder);

        download_file(url, &archive_path).await?;
        let tar_gz = File::open(&archive_path).map_err(|_| CloudError::FileError)?;
        let decompressor = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(decompressor);

        let temp_extract = format!("{}/temp_extract", local);
        create_dir_all(&temp_extract).map_err(|_| CloudError::FileError)?;
        archive
            .unpack(&temp_extract)
            .map_err(|_| CloudError::FileError)?;

        let mut entries = std::fs::read_dir(&temp_extract)
            .map_err(|_| CloudError::FileError)?
            .filter_map(Result::ok);
        if let Some(first_entry) = entries.next() {
            let extracted_folder = first_entry.path();
            for entry in std::fs::read_dir(&extracted_folder).map_err(|_| CloudError::FileError)? {
                let entry = entry.map_err(|_| CloudError::FileError)?;
                let target = Path::new(&local).join(entry.file_name());
                std::fs::rename(entry.path(), target).map_err(|_| CloudError::FileError)?;
            }
        }

        std::fs::remove_dir_all(&temp_extract).map_err(|_| CloudError::FileError)?;
        remove_file(&archive_path)
            .await
            .map_err(|_| CloudError::FileError)?;

        Ok(())
    }
    pub fn is_installed(&self) -> bool {
        let java_bin = format!("{}/bin/java", self.local_path());
        Path::new(&java_bin).exists()
    }
}

pub async fn stop_screen(inst_arc: Arc<Mutex<Instance>>) -> Result<(), CloudError> {
    let mut instance = inst_arc.lock().await;
    let status = Command::new("screen")
        .arg("-S")
        .arg(instance.server_id.clone())
        .arg("-X")
        .arg("stuff")
        .arg("stop\n")
        .status()
        .map_err(|_| CloudError::ScreenError)?;
    if status.success() {
        instance.heartbeat_started = false;
        Ok(())
    } else {
        Err(CloudError::ScreenError)
    }
}

pub async fn start_screen(instance: Instance) -> Result<(), CloudError> {
    let loader = build_loader(&instance.loader);
    let java_version = loader.java_version();

    if !java_version.is_installed() {
        java_version.install().await?
    }

    let java_path = format!("../../../{}/bin/java", java_version.local_path());
    let mut cmd = Command::new("screen");
    cmd.arg("-S")
        .arg(&instance.server_id)
        .arg("-dm")
        .arg(java_path)
        .arg("-jar")
        .arg(format!(
            "../../../versions/{}/{}-{}.jar",
            loader.name(),
            loader.name(),
            loader.version().get()
        ))
        .arg("nogui")
        .current_dir(format!(
            "running/{}/{}",
            if instance.is_persistent {
                "static"
            } else {
                "disposable"
            },
            instance.server_id
        ));

    let status = cmd.status().map_err(|_| CloudError::ScreenError)?;
    if status.success() {
        Ok(())
    } else {
        Err(CloudError::ScreenError)
    }
}
