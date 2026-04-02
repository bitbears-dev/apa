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
use crate::planner::{Planner, RiskLevel};

use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{FuzzySelect, Select, theme::ColorfulTheme};
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

async fn generate_plan_interactively(
    planner: &Planner,
    original_prompt: &str,
    profile: &str,
    region: &str,
) -> anyhow::Result<(crate::planner::Plan, String)> {
    let mut current_prompt = original_prompt.to_string();
    loop {
        let plan = planner
            .generate_plan(&current_prompt, profile, region)
            .await?;

        let mut missing_found = false;
        if let Some(ref missing) = plan.missing_parameters
            && !missing.is_empty()
        {
            missing_found = true;
            println!("\n{}", "Required information is missing:".yellow().bold());
            let mut added_info = String::new();
            for param in missing {
                let input: String = dialoguer::Input::new()
                    .with_prompt(format!("{} ({})", param.name.cyan(), param.description))
                    .interact_text()
                    .expect("Failed to read input");

                added_info.push_str(&format!("- {}: {}\n", param.name, input));
            }
            current_prompt.push_str("\n\nUser provided the following missing information:\n");
            current_prompt.push_str(&added_info);
        }

        if !missing_found {
            return Ok((plan, current_prompt));
        }
    }
}

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
    let mut aws_context = AwsContext::detect();

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
                match generate_plan_interactively(
                    &planner,
                    &prompt_text,
                    &aws_context.profile,
                    &aws_context.region,
                )
                .await
                {
                    Ok((plan, final_prompt)) => {
                        ui::print_plan(&plan);
                        let _ = history_mgr.append(&HistoryRecord {
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .to_string(),
                            prompt: final_prompt,
                            plan: Some(plan),
                            exit_code: None,
                        });
                    }
                    Err(e) => {
                        println!("{}", format!("Failed to generate plan: {}", e).red());
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
                match generate_plan_interactively(
                    &planner,
                    &prompt_text,
                    &aws_context.profile,
                    &aws_context.region,
                )
                .await
                {
                    Ok((plan, final_prompt)) => {
                        ui::print_plan(&plan);

                        let mut exit_code = None;
                        if PolicyEngine::validate(&plan).unwrap_or(false) {
                            loop {
                                let default_sel = if plan.risk_level == RiskLevel::Low { 0 } else { 2 };
                                let items = vec!["Execute command", "Change AWS Profile / Region", "Cancel execution"];
                                
                                println!();
                                let selection = Select::with_theme(&ColorfulTheme::default())
                                    .with_prompt("What do you want to do?")
                                    .default(default_sel)
                                    .items(&items)
                                    .interact()
                                    .unwrap_or(2);

                                match selection {
                                    0 => { // Execute
                                        match Executor::run(&plan, &aws_context.profile, &aws_context.region) {
                                            Ok(code) => {
                                                exit_code = code;
                                            }
                                            Err(e) => {
                                                println!("{}", format!("Execution failed: {}", e).red());
                                            }
                                        }
                                        break;
                                    }
                                    1 => { // Change Context
                                        let profiles = AwsContext::list_profiles();
                                        if let Ok(idx) = FuzzySelect::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select AWS Profile")
                                            .default(profiles.iter().position(|r| r == &aws_context.profile).unwrap_or(0))
                                            .items(&profiles)
                                            .interact()
                                        {
                                            aws_context.profile = profiles[idx].clone();
                                        }

                                        let regions = AwsContext::list_regions();
                                        if let Ok(idx) = FuzzySelect::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select AWS Region")
                                            .default(regions.iter().position(|r| r == &aws_context.region).unwrap_or(0))
                                            .items(&regions)
                                            .interact()
                                        {
                                            aws_context.region = regions[idx].clone();
                                        }
                                        
                                        println!("{}", format!("Context updated: Profile='{}', Region='{}'", aws_context.profile, aws_context.region).green());
                                    }
                                    _ => { // Cancel
                                        println!("{}", "Execution cancelled by user.".yellow());
                                        exit_code = Some(130);
                                        break;
                                    }
                                }
                            }
                        } else {
                            println!("{}", "Execution cancelled by policy.".yellow());
                            exit_code = Some(130);
                        }

                        let _ = history_mgr.append(&HistoryRecord {
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .to_string(),
                            prompt: final_prompt,
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
