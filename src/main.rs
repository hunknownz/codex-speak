mod bundled;
mod codex_integration;
mod config;
mod control_app;
mod controls;
mod doctor;
mod extract;
mod install;
mod mcp;
mod model_catalog;
mod pet_state;
mod process;
mod pronunciation;
mod release_manifest;
mod session;
mod settings;
mod side_channel;
mod status;
mod support;
mod tts;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "codex-speak")]
#[command(about = "Local Chinese-first speech helper for Codex replies")]
#[command(version)]
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
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Verify the installed app layout and Codex integration after setup.
    VerifyInstall {
        #[arg(long)]
        allow_missing_models: bool,
    },
    /// Verify Skill/MCP side-channel and hook consumption behavior without playing audio.
    VerifyCodex {
        #[arg(long)]
        json: bool,
    },
    /// Verify settings controls can be changed, observed, and restored.
    VerifyControls {
        #[arg(long)]
        json: bool,
    },
    /// Verify an unpacked release package manifest and key file hashes.
    VerifyPackage {
        #[arg(long)]
        package_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Print machine-readable status for plugins and scripts.
    Status,
    /// Write a local support bundle for installation or playback troubleshooting.
    SupportBundle {
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        include_private: bool,
    },
    /// Print the desktop pet state as JSON.
    PetState,
    /// Open or locate the installed control app.
    App {
        #[command(subcommand)]
        command: AppCommand,
    },
    /// Read or update Codex Speak configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Manage local pronunciation replacements for speech-friendly terms.
    Pronunciation {
        #[command(subcommand)]
        command: PronunciationCommand,
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
        #[arg(long)]
        no_summary: bool,
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
enum PronunciationCommand {
    /// Print the local pronunciation dictionary path.
    Path,
    /// Print the local pronunciation dictionary as JSON.
    List,
    /// Add or update a pronunciation replacement.
    Set {
        #[arg(long)]
        term: String,
        #[arg(long)]
        spoken: String,
    },
    /// Remove a pronunciation replacement.
    Remove {
        #[arg(long)]
        term: String,
    },
    /// Preview speech normalization with the local dictionary applied.
    Preview {
        #[arg(long)]
        text: String,
    },
}

#[derive(Debug, Subcommand)]
enum ModelsCommand {
    /// Print supported local TTS providers and their install status.
    List,
    /// Install the current provider model, a named provider, or every supported local model.
    Install {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AppCommand {
    /// Open the installed Tauri control app.
    Open,
    /// Print the expected installed control app path.
    Path,
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
        Command::Doctor { json } => doctor::run(json)?,
        Command::VerifyInstall {
            allow_missing_models,
        } => doctor::verify_install(allow_missing_models)?,
        Command::VerifyCodex { json } => codex_integration::run(json)?,
        Command::VerifyControls { json } => controls::run(json)?,
        Command::VerifyPackage { package_dir, json } => {
            release_manifest::run(package_dir.as_deref(), json)?
        }
        Command::Status => {
            let cfg = config::Config::load_or_default()?;
            println!("{}", serde_json::to_string_pretty(&status::collect(&cfg)?)?);
        }
        Command::SupportBundle {
            output,
            include_private,
        } => {
            let dir = support::write_bundle(
                output.as_deref(),
                support::SupportBundleOptions { include_private },
            )?;
            println!("{}", dir.display());
        }
        Command::PetState => println!(
            "{}",
            serde_json::to_string_pretty(&pet_state::read_state()?)?
        ),
        Command::App { command } => match command {
            AppCommand::Open => control_app::open()?,
            AppCommand::Path => control_app::print_path()?,
        },
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
        Command::Pronunciation { command } => match command {
            PronunciationCommand::Path => {
                println!("{}", pronunciation::dictionary_path()?.display())
            }
            PronunciationCommand::List => {
                let dictionary = pronunciation::load_user_dictionary()?;
                println!("{}", serde_json::to_string_pretty(&dictionary)?);
            }
            PronunciationCommand::Set { term, spoken } => {
                let update = pronunciation::set_user_term(&term, &spoken)?;
                println!("{}", serde_json::to_string_pretty(&update)?);
            }
            PronunciationCommand::Remove { term } => {
                let update = pronunciation::remove_user_term(&term)?;
                println!("{}", serde_json::to_string_pretty(&update)?);
            }
            PronunciationCommand::Preview { text } => {
                let dictionary = pronunciation::load_user_dictionary()?;
                println!(
                    "{}",
                    pronunciation::normalize_for_tts_with_dictionary(&text, &dictionary)
                );
            }
        },
        Command::Models { command } => match command {
            ModelsCommand::List => {
                let providers = status::provider_statuses()?;
                println!("{}", serde_json::to_string_pretty(&providers)?);
            }
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
        Command::Install {
            skip_tts_download,
            no_summary,
        } => install::install(skip_tts_download, no_summary)?,
        Command::Uninstall { remove_models } => install::uninstall(remove_models)?,
    }

    Ok(())
}
