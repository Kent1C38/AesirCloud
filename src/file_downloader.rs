use crate::errors::CloudError;

use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn download_file(url: &str, path: &str) -> Result<(), CloudError> {
    let response = reqwest::get(url)
        .await
        .map_err(|_| CloudError::DownloadError)?;

    let mut file = File::create(path)
        .await
        .map_err(|_| CloudError::FileError)?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| CloudError::DownloadError)?;
        file.write_all(&chunk)
            .await
            .map_err(|_| CloudError::FileError)?;
    }

    Ok(())
}
