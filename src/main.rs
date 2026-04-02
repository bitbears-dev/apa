mod config;
mod context;
mod executor;
mod history;
mod planner;
mod ui;

use crate::config::AppConfig;
use crate::context::AwsContext;
use crate::executor::{Executor, PolicyEngine};
use crate::history::{HistoryManager, HistoryRecord};
use crate::planner::Planner;

use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use colored::*;
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "apa", version, about = "APA: AI Powered AWS CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The natural language prompt (if no subcommand is provided)
    #[arg(global = false, default_value = "")]
    prompt: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show the execution plan without running it
    Plan {
        /// The natural language prompt
        prompt: Vec<String>,
    },
    /// Explicitly execute a prompt (still subject to policy)
    Exec {
        /// The natural language prompt
        prompt: Vec<String>,
    },
    /// Manage configuration
    Config,
    /// Show execution history
    History,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Load components
    let app_config = AppConfig::load();
    let aws_context = AwsContext::detect();

    debug!(
        "Loaded Config (API key presence: {})",
        app_config.openai_api_key.is_some()
    );
    info!(
        "Running with AWS Context: Profile='{}', Region='{}'",
        aws_context.profile, aws_context.region
    );

    let history_mgr = HistoryManager::new();

    match &cli.command {
        Some(Commands::Plan { prompt }) => {
            let prompt_text = prompt.join(" ");
            info!("Planning instruction: {}", prompt_text);

            if let Some(api_key) = &app_config.openai_api_key {
                let planner = Planner::new(api_key.clone());
                match planner
                    .generate_plan(&prompt_text, &aws_context.profile, &aws_context.region)
                    .await
                {
                    Ok(plan) => {
                        ui::print_plan(&plan);
                        let _ = history_mgr.append(&HistoryRecord {
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .to_string(),
                            prompt: prompt_text,
                            plan: Some(plan),
                            exit_code: None,
                        });
                    }
                    Err(e) => {
                        println!("Failed to generate plan: {}", e);
                    }
                }
            } else {
                println!("Error: OPENAI_API_KEY is not set in environment or config.toml.");
            }
        }
        Some(Commands::Exec { prompt }) => {
            let prompt_text = prompt.join(" ");
            info!("Executing instruction: {}", prompt_text);
            // TODO: impl exec with policy
            println!("Exec with policy will be implemented in Phase 2");
        }
        Some(Commands::Config) => {
            info!("Config command");
            // TODO: impl config management
        }
        Some(Commands::History) => {
            info!("History command");
            // TODO: impl history viewing
        }
        None => {
            if cli.prompt.is_empty() {
                // If no prompt is provided and no subcommand, print help
                // By default `clap` will just do nothing, we should invoke help.
                // An easy way here is to print a message instead.
                println!("No prompt provided. Try `apa --help` for usage.");
                return Ok(());
            }

            let prompt_text = cli.prompt;
            info!("Default acting on instruction: {}", prompt_text);

            if let Some(api_key) = &app_config.openai_api_key {
                let planner = Planner::new(api_key.clone());
                match planner
                    .generate_plan(&prompt_text, &aws_context.profile, &aws_context.region)
                    .await
                {
                    Ok(plan) => {
                        ui::print_plan(&plan);

                        let mut exit_code = None;
                        if PolicyEngine::validate_and_confirm(&plan).unwrap_or(false) {
                            match Executor::run(&plan, &aws_context.profile, &aws_context.region) {
                                Ok(code) => {
                                    exit_code = code;
                                }
                                Err(e) => {
                                    println!("{}", format!("Execution failed: {}", e).red());
                                }
                            }
                        } else {
                            println!("{}", "Execution cancelled by policy or user.".yellow());
                            exit_code = Some(130);
                        }

                        let _ = history_mgr.append(&HistoryRecord {
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .to_string(),
                            prompt: prompt_text.clone(),
                            plan: Some(plan),
                            exit_code,
                        });
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to generate plan: {}", e).red());
                    }
                }
            } else {
                println!("Error: OPENAI_API_KEY is not set.");
            }
        }
    }

    Ok(())
}
