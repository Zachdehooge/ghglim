use chrono::{DateTime, Local, TimeZone, Utc};
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

fn parse_github_timestamp_to_local(timestamp: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    // First parse as UTC, then convert to local
    let utc_dt = DateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S%.3fZ")
        .or_else(|_| DateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%SZ"))
        .or_else(|_| {
            // If Z format fails, try with timezone offset
            let normalized = timestamp.replace("Z", "+00:00");
            DateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%.3f%z")
        })
        .or_else(|_| {
            let normalized = timestamp.replace("Z", "+00:00");
            DateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%z")
        })?;

    // Convert to UTC first, then to local
    let utc_datetime = Utc.from_utc_datetime(&utc_dt.naive_utc());
    Ok(utc_datetime.with_timezone(&Local))
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

        // Parse and format created date in local time
        match parse_github_timestamp_to_local(&workflow.created_at) {
            Ok(created_dt) => {
                println!("🎉 Created: {}", created_dt.format("%m-%d-%Y at %H:%M"))
            }
            Err(_) => {
                println!("🎉 Created: {} (raw format)", workflow.created_at);
                eprintln!(
                    "⚠️  Could not parse created_at date format: {}",
                    workflow.created_at
                );
            }
        }

        // Check if active
        let is_active = workflow.state == "active";
        println!(
            "🔄 Is Active: {}",
            if is_active { "Yes ✅" } else { "No ❌" }
        );

        // Parse and format updated date in local time
        match parse_github_timestamp_to_local(&workflow.updated_at) {
            Ok(updated_dt) => println!(
                "📅 Last Updated: {}",
                updated_dt.format("%m-%d-%Y at %H:%M")
            ),
            Err(_) => {
                println!("📅 Last Updated: {} (raw format)", workflow.updated_at);
                eprintln!(
                    "⚠️  Could not parse updated_at date format: {}",
                    workflow.updated_at
                );
            }
        }

        print!("🏃 Last Run: ");
        match get_last_run_date(client, owner, repo, workflow.id) {
            Ok(Some(last_run_date)) => match parse_github_timestamp_to_local(&last_run_date) {
                Ok(run_dt) => println!("{}", run_dt.format("%m-%d-%Y at %H:%M")),
                Err(_) => {
                    println!("{} (raw format)", last_run_date);
                    eprintln!(
                        "⚠️  Could not parse last run date format: {}",
                        last_run_date
                    );
                }
            },
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

        display_workflows(&json, &client, &args.owner, &args.repo)?;
    } else {
        println!("❌ Request failed with status: {}", response.status());
    }

    Ok(())
}
