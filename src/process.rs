use std::process::Command;

use anyhow::Result;

pub fn stop_speech() -> Result<()> {
    if cfg!(target_os = "macos") {
        let _ = Command::new("/usr/bin/pkill")
            .args(["-x", "afplay"])
            .status();
        let _ = Command::new("/usr/bin/pkill").args(["-x", "say"]).status();
        let _ = Command::new("/usr/bin/pkill")
            .args(["-f", "sherpa-onnx-offline-tts"])
            .status();
    } else if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/IM", "afplay.exe", "/F"])
            .status();
        let _ = Command::new("taskkill")
            .args(["/IM", "sherpa-onnx-offline-tts.exe", "/F"])
            .status();
    }
    Ok(())
}
