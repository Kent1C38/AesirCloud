use std::process::Command;

pub fn stop_screen(screen_name: String) -> Result<(), String> {
    let status = Command::new("screen")
        .arg("-S")
        .arg(screen_name)
        .arg("-X")
        .arg("stuff")
        .arg("stop\n")
        .status()
        .map_err(|e| format!("Error while executing command: {:?}", e))?;
    match status.success() {
        true => Ok(()),
        false => Err(format!(
            "\'screen\' command failed with code {}",
            status.code().unwrap_or(-1)
        )),
    }
}
