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

        let mut cmd = Command::new("aws");
        cmd.args(&actual_args);

        cmd.env("AWS_PROFILE", profile);
        cmd.env("AWS_REGION", region);

        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        println!("{}", format!("Executing: aws {} (with AWS_PROFILE={} AWS_REGION={})", shell_words::join(actual_args).as_str(), profile, region).cyan());
        let mut child = cmd.spawn()?;
        let status = child.wait()?;

        Ok(status.code())
    }
}
