mod config;
mod doctor;
mod extract;
mod install;
mod mcp;
mod process;
mod session;
mod settings;
mod side_channel;
mod status;
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
    /// Print machine-readable status for plugins and scripts.
    Status,
    /// Read or update Codex Speak configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Download or repair local TTS models.
    Models {
        #[command(subcommand)]
        command: ModelsCommand,
    },
    /// Run the Codex Speak MCP server for the Codex plugin.
    Mcp,
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

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Print the current configuration as JSON.
    Get,
    /// Update one or more configuration fields.
    Set {
        #[arg(long)]
        enabled: Option<bool>,
        #[arg(long)]
        child_mode: Option<bool>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        speed: Option<f32>,
        #[arg(long)]
        max_read_chars: Option<usize>,
        #[arg(long)]
        voice_profile: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ModelsCommand {
    /// Install the current provider model, a named provider, or every supported local model.
    Install {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        all: bool,
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
            let extracted = session::resolve_text_for_speech(text, fixture.as_deref(), &cfg)?;
            tts::speak(&cfg, &extracted, no_play)?;
        }
        Command::Stop => process::stop_speech()?,
        Command::Doctor => doctor::run()?,
        Command::Status => {
            let cfg = config::Config::load_or_default()?;
            println!("{}", serde_json::to_string_pretty(&status::collect(&cfg)?)?);
        }
        Command::Config { command } => match command {
            ConfigCommand::Get => {
                let cfg = config::Config::load_or_default()?;
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigCommand::Set {
                enabled,
                child_mode,
                provider,
                speed,
                max_read_chars,
                voice_profile,
            } => {
                let cfg = config::Config::load_or_default()?;
                let update = settings::apply_patch(
                    cfg,
                    settings::ConfigPatch {
                        enabled,
                        child_mode,
                        provider,
                        speed,
                        max_read_chars,
                        voice_profile,
                    },
                )?;
                update.config.save()?;
                println!("{}", serde_json::to_string_pretty(&update)?);
            }
        },
        Command::Models { command } => match command {
            ModelsCommand::Install { provider, all } => {
                if all {
                    install::install_all_models()?;
                } else {
                    let provider = match provider {
                        Some(provider) => provider,
                        None => config::Config::load_or_default()?.provider,
                    };
                    install::install_model(&provider)?;
                }
                let cfg = config::Config::load_or_default()?;
                println!("{}", serde_json::to_string_pretty(&status::collect(&cfg)?)?);
            }
        },
        Command::Mcp => mcp::run()?,
        Command::Install { skip_tts_download } => install::install(skip_tts_download)?,
        Command::Uninstall { remove_models } => install::uninstall(remove_models)?,
    }

    Ok(())
}
