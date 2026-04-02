use crate::planner::{Plan, RiskLevel};
use colored::*;

pub fn print_plan(plan: &Plan) {
    println!("\n{}", "=== Execution Plan ===".cyan().bold());
    println!("{}: {}", "Intent".bold(), plan.intent_summary);

    let risk_color = match plan.risk_level {
        RiskLevel::Low => "green",
        RiskLevel::Medium => "yellow",
        RiskLevel::High => "red",
    };

    // {:?} prints "Low", "Medium", "High"
    println!(
        "{}: {}",
        "Risk Level".bold(),
        format!("{:?}", plan.risk_level).color(risk_color)
    );

    if let Some(profile) = &plan.profile {
        println!("{}: {}", "Profile".bold(), profile);
    }
    if let Some(region) = &plan.region {
        println!("{}: {}", "Region".bold(), region);
    }

    println!("\n{}:", "Command".bold());
    let mut args_clone = vec!["aws".to_string()];
    args_clone.extend(plan.aws_cli_args.clone());
    println!("  {}", args_clone.join(" ").bright_white());

    if !plan.assumptions.is_empty() {
        println!("\n{}:", "Assumptions / Warnings".yellow().bold());
        for assumption in &plan.assumptions {
            println!("  - {}", assumption);
        }
    }

    println!("\n{}: {}", "Explanation".bold(), plan.explanation);
    println!("======================\n");
}
