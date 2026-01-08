#[derive(Debug)]
pub enum CloudError {
    FileError,
    FatalError,
    ScreenError,
    DownloadError,
    UnavailablePort,
    InstanceAlreadyExists,
    HTTPError,
    JSONError,
    NoStableBuild,
}
