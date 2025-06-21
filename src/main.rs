use chrono::DateTime;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Deserialize, Serialize)]
struct Workflow {
    id: u64,
    name: String,
    state: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkflowResponse {
    total_count: i32,
    workflows: Vec<Workflow>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkflowRun {
    created_at: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkflowRunsResponse {
    total_count: i32,
    workflow_runs: Vec<WorkflowRun>,
}

/// Simple program to fetch and display GitHub workflows
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Repository owner
    #[arg(short, long)]
    owner: String,
    /// Repository name
    #[arg(short, long)]
    repo: String,
}

fn get_last_run_date(
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
    workflow_id: u64,
) -> Result<Option<String>, Box<dyn Error>> {
    let response = client
        .get(format!(
            "https://api.github.com/repos/{}/{}/actions/workflows/{}/runs?per_page=1",
            owner, repo, workflow_id
        ))
        .header("User-Agent", "MyApp/1.0")
        .send()?;

    if response.status().is_success() {
        let json: Value = response.json()?;
        let runs_response: WorkflowRunsResponse = serde_json::from_value(json)?;

        if let Some(last_run) = runs_response.workflow_runs.first() {
            Ok(Some(last_run.created_at.clone()))
        } else {
            Ok(None)
        }
    } else {
        println!(
            "⚠️  Failed to fetch runs for workflow {}: {}",
            workflow_id,
            response.status()
        );
        Ok(None)
    }
}

fn display_workflows(
    json: &Value,
    client: &reqwest::blocking::Client,
    owner: &str,
    repo: &str,
) -> Result<(), Box<dyn Error>> {
    let response: WorkflowResponse = serde_json::from_value(json.clone())?;

    println!("🔧 GitHub Workflows Summary");
    println!("═══════════════════════════");
    println!("📊 Total workflows: {}\n", response.total_count);

    for (index, workflow) in response.workflows.iter().enumerate() {
        println!("🚀 Workflow #{}", index + 1);
        println!("📝 Name: {}", workflow.name);

        // State with emoji
        let state_emoji = match workflow.state.as_str() {
            "active" => "✅",
            "disabled" => "❌",
            _ => "❓",
        };
        println!("{} State: {}", state_emoji, workflow.state);

        // Parse and format created date
        match DateTime::parse_from_str(&workflow.created_at, "%Y-%m-%dT%H:%M:%S%.3f%z") {
            Ok(created_dt) => {
                println!("🎂 Created: {}", created_dt.format("%B %d, %Y at %I:%M %p"))
            }
            Err(_) => println!("🎂 Created: {}", workflow.created_at),
        }

        // Check if active
        let is_active = workflow.state == "active";
        println!(
            "🔄 Is Active: {}",
            if is_active { "Yes ✅" } else { "No ❌" }
        );

        // Parse and format last updated date
        match DateTime::parse_from_str(&workflow.updated_at, "%Y-%m-%dT%H:%M:%S%.3f%z") {
            Ok(updated_dt) => println!(
                "📅 Last Updated: {}",
                updated_dt.format("%B %d, %Y at %I:%M %p")
            ),
            Err(_) => println!("📅 Last Updated: {}", workflow.updated_at),
        }

        // Get and display last run date
        print!("🏃 Last Run: ");
        match get_last_run_date(client, owner, repo, workflow.id) {
            Ok(Some(last_run_date)) => {
                // Handle the 'Z' suffix by replacing it with '+00:00' for proper timezone parsing
                let normalized_date = last_run_date.replace("Z", "+00:00");

                let parsed_date = DateTime::parse_from_str(&normalized_date, "%Y-%m-%dT%H:%M:%S%z")
                    .or_else(|_| {
                        DateTime::parse_from_str(&normalized_date, "%Y-%m-%dT%H:%M:%S%.3f%z")
                    })
                    .or_else(|_| DateTime::parse_from_str(&last_run_date, "%Y-%m-%dT%H:%M:%SZ"))
                    .or_else(|_| {
                        DateTime::parse_from_str(&last_run_date, "%Y-%m-%dT%H:%M:%S%.3fZ")
                    });

                match parsed_date {
                    Ok(run_dt) => println!("{}", run_dt.format("%B %d, %Y at %I:%M %p")),
                    Err(_) => {
                        println!("{} (raw format)", last_run_date);
                        eprintln!("⚠️  Could not parse date format: {}", last_run_date);
                    }
                }
            }
            Ok(None) => println!("Never run ⏸️"),
            Err(e) => println!("Error fetching run data: {} ❌", e),
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let client = reqwest::blocking::Client::new();

    println!(
        "🔍 Fetching workflows for {}/{}...\n",
        args.owner, args.repo
    );

    let response = client
        .get(format!(
            "https://api.github.com/repos/{}/{}/actions/workflows",
            args.owner, args.repo
        ))
        .header("User-Agent", "MyApp/1.0")
        .send()?;

    if response.status().is_success() {
        let json: Value = response.json()?;

        // Display formatted workflows with last run information
        display_workflows(&json, &client, &args.owner, &args.repo)?;

        // Uncomment the line below if you also want to see the raw JSON
        // println!("\n🔍 Raw JSON Response:\n{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("❌ Request failed with status: {}", response.status());
    }

    Ok(())
}
