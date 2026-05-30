mod config;
mod doctor;
mod extract;
mod install;
mod process;
mod session;
mod tts;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "codex-speak")]
#[command(about = "Local Chinese-first speech helper for Codex replies")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Extract speakable text from text, a session fixture, or the latest Codex session.
    Extract {
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        fixture: Option<PathBuf>,
    },
    /// Speak text, a session fixture, or the latest Codex final answer.
    Speak {
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        fixture: Option<PathBuf>,
        #[arg(long)]
        no_play: bool,
    },
    /// Stop current speech playback.
    Stop,
    /// Check installation, model, player, and Codex hook state.
    Doctor,
    /// Install Codex Speak into the current user's Codex home.
    Install {
        #[arg(long)]
        skip_tts_download: bool,
    },
    /// Uninstall Codex Speak and restore previous notify when available.
    Uninstall {
        #[arg(long)]
        remove_models: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract { text, fixture } => {
            let cfg = config::Config::load_or_default()?;
            let extracted = session::resolve_text(text, fixture.as_deref(), &cfg)?;
            println!("{extracted}");
        }
        Command::Speak {
            text,
            fixture,
            no_play,
        } => {
            let cfg = config::Config::load_or_default()?;
            let extracted = session::resolve_text(text, fixture.as_deref(), &cfg)?;
            tts::speak(&cfg, &extracted, no_play)?;
        }
        Command::Stop => process::stop_speech()?,
        Command::Doctor => doctor::run()?,
        Command::Install { skip_tts_download } => install::install(skip_tts_download)?,
        Command::Uninstall { remove_models } => install::uninstall(remove_models)?,
    }

    Ok(())
}
