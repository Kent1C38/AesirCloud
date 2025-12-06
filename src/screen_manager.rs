use crate::errors::CloudError;
use std::process::Command;

pub fn stop_screen(screen_name: String) -> Result<(), CloudError> {
    let status = Command::new("screen")
        .arg("-S")
        .arg(screen_name)
        .arg("-X")
        .arg("stuff")
        .arg("stop\n")
        .status()
        .map_err(|_| CloudError::ScreenError)?;
    if status.success() {
        Ok(())
    } else {
        Err(CloudError::ScreenError)
    }
}
