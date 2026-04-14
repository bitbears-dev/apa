use crate::planner::{Plan, RiskLevel};
use anyhow::Result;
use colored::*;
use std::process::{Command, Stdio};

pub struct PolicyEngine;

impl PolicyEngine {
    pub fn validate(plan: &Plan) -> Result<bool> {
        // ガードレール定義
        let illegal_chars = ['|', '>', '<', '&', ';', '`', '$'];
        for arg in &plan.aws_cli_args {
            if arg.contains(&illegal_chars[..]) {
                println!("{}", "Error: Plan contains illegal shell characters.".red());
                return Ok(false);
            }
        }

        if plan.risk_level == RiskLevel::High {
            println!("\n{}", "Policy Block: This high-risk destructive action cannot be executed autonomously in the MVP. Plan preview only.".red().bold());
            return Ok(false);
        }

        Ok(true)
    }
}

pub struct Executor;

impl Executor {
    pub fn run(plan: &Plan, profile: &str, region: &str) -> Result<Option<i32>> {
        let mut actual_args = plan.aws_cli_args.clone();

        if actual_args.first().map(|s| s.as_str()) == Some("aws") {
            actual_args.remove(0);
        }

        // Fallback: If LLM generated a single flat string despite prompt warnings, safely split it.
        if actual_args.len() == 1 && actual_args[0].contains(" ") {
            actual_args = shell_words::split(&actual_args[0]).unwrap_or(actual_args);
        }

        if !actual_args.contains(&"--profile".to_string()) && !profile.is_empty() {
            actual_args.push("--profile".to_string());
            actual_args.push(profile.to_string());
        }

        if !actual_args.contains(&"--region".to_string()) && !region.is_empty() {
            actual_args.push("--region".to_string());
            actual_args.push(region.to_string());
        }

        let mut cmd = Command::new("aws");
        cmd.args(&actual_args);

        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let actual_command = format!("aws {}", shell_words::join(actual_args.clone()));
        println!("{}", format!("Executing: {}", actual_command).cyan());

        let shell_hist_res = crate::history::append_to_shell_history(&actual_command);
        if let Err(e) = shell_hist_res {
            eprintln!(
                "{}",
                format!("Warning: Failed to append to shell history: {}", e).yellow()
            );
        }

        let mut child = cmd.spawn()?;
        let status = child.wait()?;

        Ok(status.code())
    }
}
