//! CLI tool for detecting train stop pattern changes
//!
//! Usage:
//!   cargo run --bin detect_stop_patterns
//!
//! Environment variables:
//!   ODPT_API_KEY: Required. API key for ODPT API.
//!   DATABASE_URL: Required. PostgreSQL connection string.

use sqlx::postgres::PgPoolOptions;
use stationapi::stop_pattern::{
    odpt_client::OdptOperator, GitHubIssueCreator, RotationConfig, StopPatternDetector,
};
use std::env;
use tracing::{error, info};

/// Exit codes:
/// - 0: No changes detected
/// - 1: Error occurred
/// - 10: Changes detected
const EXIT_NO_CHANGES: i32 = 0;
const EXIT_ERROR: i32 = 1;
const EXIT_CHANGES_DETECTED: i32 = 10;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(has_changes) => {
            if has_changes {
                std::process::exit(EXIT_CHANGES_DETECTED);
            } else {
                std::process::exit(EXIT_NO_CHANGES);
            }
        }
        Err(e) => {
            error!("Error: {}", e);
            std::process::exit(EXIT_ERROR);
        }
    }
}

async fn run() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    // Load .env.local if available
    if dotenv::from_filename(".env.local").is_err() {
        tracing::warn!("Could not load .env.local");
    }

    // Get API key
    let api_key = match env::var("ODPT_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            error!("ODPT_API_KEY environment variable is required");
            eprintln!("Error: ODPT_API_KEY environment variable is required");
            eprintln!();
            eprintln!("Usage:");
            eprintln!("  ODPT_API_KEY=your_key cargo run --bin detect_stop_patterns");
            eprintln!();
            eprintln!("To get an API key, register at: https://developer.odpt.org/");
            std::process::exit(1);
        }
    };

    // Connect to database
    let db_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            error!("DATABASE_URL environment variable is required");
            eprintln!("Error: DATABASE_URL environment variable is required");
            eprintln!();
            eprintln!("Usage:");
            eprintln!(
                "  DATABASE_URL=postgres://user:pass@host/db cargo run --bin detect_stop_patterns"
            );
            eprintln!();
            eprintln!("The DATABASE_URL should be a valid PostgreSQL connection string.");
            std::process::exit(1);
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    info!("Connected to database");

    // Parse operators and rotation config from command line
    let operators = parse_operators();
    let rotation_config = parse_rotation_config();

    info!("Detecting stop patterns for {} operators", operators.len());
    for op in &operators {
        info!("  - {} ({})", op.name(), op.id());
    }

    info!(
        "Rotation: changes_retention={}d, snapshots_retention={}d, auto={}",
        rotation_config.changes_retention_days,
        rotation_config.snapshots_retention_days,
        rotation_config.auto_rotate
    );

    // Create detector and run
    let detector =
        StopPatternDetector::new(api_key, pool).with_rotation_config(rotation_config.clone());
    let changes = detector.detect_changes(&operators).await?;

    // Output results
    let output = StopPatternDetector::format_changes(&changes);
    println!("{}", output);

    let has_changes = !changes.is_empty();

    if has_changes {
        info!("Changes have been saved to the database");
        info!("Review with: SELECT * FROM stop_pattern_changes WHERE acknowledged = FALSE;");

        // Create GitHub issue if configured
        if let Some((token, repo)) = get_github_config() {
            info!("Creating GitHub issue for detected changes...");
            let creator = GitHubIssueCreator::new(token, repo);
            match creator.create_issue(&changes).await {
                Ok(url) => info!("GitHub issue created: {}", url),
                Err(e) => error!("Failed to create GitHub issue: {}", e),
            }
        }
    }

    Ok(has_changes)
}

fn get_github_config() -> Option<(String, String)> {
    let args: Vec<String> = env::args().collect();

    // Check for --github-issue flag
    if !args.iter().any(|a| a == "--github-issue") {
        return None;
    }

    // Get token and repo from environment variables
    let token = env::var("GITHUB_TOKEN").ok()?;
    let repo = env::var("GITHUB_REPO").ok()?;

    Some((token, repo))
}

fn parse_operators() -> Vec<OdptOperator> {
    let args: Vec<String> = env::args().collect();

    // Check for --help
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        std::process::exit(0);
    }

    // Check for --operators argument
    for i in 0..args.len() {
        if (args[i] == "--operators" || args[i] == "-o") && i + 1 < args.len() {
            return parse_operator_list(&args[i + 1]);
        }
    }

    // Check for OPERATORS environment variable
    if let Ok(operators_env) = env::var("OPERATORS") {
        if !operators_env.is_empty() {
            info!("Using OPERATORS from environment variable");
            return parse_operator_list(&operators_env);
        }
    }

    // Default: Tokyo Metro and Toei only (for faster initial testing)
    vec![OdptOperator::TokyoMetro, OdptOperator::Toei]
}

fn parse_operator_list(list: &str) -> Vec<OdptOperator> {
    if list == "all" {
        return OdptOperator::all();
    }

    list.split(',')
        .filter_map(|s| match s.trim().to_lowercase().as_str() {
            "tokyometro" | "tokyo-metro" | "metro" => Some(OdptOperator::TokyoMetro),
            "toei" => Some(OdptOperator::Toei),
            "jr-east" | "jreast" | "jr" => Some(OdptOperator::JREast),
            "tobu" => Some(OdptOperator::Tobu),
            "seibu" => Some(OdptOperator::Seibu),
            "keio" => Some(OdptOperator::Keio),
            "odakyu" => Some(OdptOperator::Odakyu),
            "tokyu" => Some(OdptOperator::Tokyu),
            "keikyu" => Some(OdptOperator::Keikyu),
            "keisei" => Some(OdptOperator::Keisei),
            "sotetsu" => Some(OdptOperator::Sotetsu),
            _ => {
                eprintln!("Warning: Unknown operator '{}', skipping", s);
                None
            }
        })
        .collect()
}

fn parse_rotation_config() -> RotationConfig {
    use tracing::warn;

    let args: Vec<String> = env::args().collect();
    let mut config = RotationConfig::default();

    for i in 0..args.len() {
        if args[i] == "--changes-retention" && i + 1 < args.len() {
            match args[i + 1].parse::<i32>() {
                Ok(days) if days > 0 => {
                    config.changes_retention_days = days;
                }
                Ok(days) => {
                    warn!(
                        "Invalid --changes-retention value '{}': must be positive, using default {}",
                        days, config.changes_retention_days
                    );
                }
                Err(_) => {
                    warn!(
                        "Failed to parse --changes-retention value '{}': not a valid integer, using default {}",
                        args[i + 1], config.changes_retention_days
                    );
                }
            }
        }
        if args[i] == "--snapshots-retention" && i + 1 < args.len() {
            match args[i + 1].parse::<i32>() {
                Ok(days) if days > 0 => {
                    config.snapshots_retention_days = days;
                }
                Ok(days) => {
                    warn!(
                        "Invalid --snapshots-retention value '{}': must be positive, using default {}",
                        days, config.snapshots_retention_days
                    );
                }
                Err(_) => {
                    warn!(
                        "Failed to parse --snapshots-retention value '{}': not a valid integer, using default {}",
                        args[i + 1], config.snapshots_retention_days
                    );
                }
            }
        }
        if args[i] == "--no-rotate" {
            config.auto_rotate = false;
        }
    }

    config
}

fn print_help() {
    println!("detect_stop_patterns - Detect train stop pattern changes from ODPT API");
    println!();
    println!("USAGE:");
    println!("    detect_stop_patterns [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --operators <LIST>  Comma-separated list of operators, or 'all'");
    println!("                            Default: TokyoMetro,Toei");
    println!("    --changes-retention <DAYS>");
    println!("                            Days to keep acknowledged changes (default: 90)");
    println!("    --snapshots-retention <DAYS>");
    println!("                            Days to keep snapshots (default: 30)");
    println!("    --no-rotate             Disable automatic log rotation");
    println!("    --github-issue          Create GitHub issue when changes detected");
    println!("                            Requires GITHUB_TOKEN and GITHUB_REPO env vars");
    println!("    -h, --help              Print this help message");
    println!();
    println!("OPERATORS:");
    println!("    TokyoMetro  - Tokyo Metro (東京メトロ)");
    println!("    Toei        - Toei Subway (都営地下鉄)");
    println!("    JR-East     - JR East (JR東日本)");
    println!("    Tobu        - Tobu Railway (東武鉄道)");
    println!("    Seibu       - Seibu Railway (西武鉄道)");
    println!("    Keio        - Keio Corporation (京王電鉄)");
    println!("    Odakyu      - Odakyu Electric Railway (小田急電鉄)");
    println!("    Tokyu       - Tokyu Corporation (東急電鉄)");
    println!("    Keikyu      - Keikyu Corporation (京急電鉄)");
    println!("    Keisei      - Keisei Electric Railway (京成電鉄)");
    println!("    Sotetsu     - Sagami Railway (相鉄)");
    println!();
    println!("EXAMPLES:");
    println!("    # Detect changes for Tokyo Metro and Toei (default)");
    println!("    detect_stop_patterns");
    println!();
    println!("    # Detect changes for specific operators");
    println!("    detect_stop_patterns -o TokyoMetro,JR-East,Tokyu");
    println!();
    println!("    # Detect changes for all operators");
    println!("    detect_stop_patterns -o all");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    ODPT_API_KEY   Required. API key for ODPT API.");
    println!("    DATABASE_URL   Required. PostgreSQL connection string.");
    println!("    OPERATORS      Optional. Comma-separated list of operators or 'all'.");
    println!("                   Used if -o/--operators is not provided.");
    println!(
        "    GITHUB_TOKEN   Required for --github-issue. Personal access token with repo scope."
    );
    println!("    GITHUB_REPO    Required for --github-issue. Repository in 'owner/repo' format.");
    println!();
    println!("To get an ODPT API key, register at: https://developer.odpt.org/");
}
