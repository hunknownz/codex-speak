use std::process::Command;

use anyhow::{Context, Result};

use crate::config;

pub fn print_path() -> Result<()> {
    println!("{}", config::control_app_path()?.display());
    Ok(())
}

pub fn open() -> Result<()> {
    let path = config::control_app_path()?;
    if !path.exists() {
        anyhow::bail!(
            "control app is not installed at {}; run the installer or use --skip-control-app only for headless installs",
            path.display()
        );
    }

    if cfg!(target_os = "macos") {
        Command::new("/usr/bin/open")
            .arg(&path)
            .status()
            .context("failed to open control app")?;
    } else if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(&path)
            .status()
            .context("failed to open control app")?;
    } else {
        Command::new("xdg-open")
            .arg(&path)
            .status()
            .context("failed to open control app")?;
    }

    Ok(())
}
