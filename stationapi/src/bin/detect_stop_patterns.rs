//! CLI tool for detecting train stop pattern changes
//!
//! Usage:
//!   cargo run --bin detect_stop_patterns
//!
//! Environment variables:
//!   ODPT_API_KEY: Required. API key for ODPT API.
//!   DATABASE_URL: Required. PostgreSQL connection string.

use sqlx::postgres::PgPoolOptions;
use stationapi::config::fetch_database_url;
use stationapi::stop_pattern::{odpt_client::OdptOperator, StopPatternDetector};
use std::env;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    let db_url = fetch_database_url();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    info!("Connected to database");

    // Parse operators from command line or use defaults
    let operators = parse_operators();

    info!("Detecting stop patterns for {} operators", operators.len());
    for op in &operators {
        info!("  - {} ({})", op.name(), op.id());
    }

    // Create detector and run
    let detector = StopPatternDetector::new(api_key, pool);
    let changes = detector.detect_changes(&operators).await?;

    // Output results
    let output = StopPatternDetector::format_changes(&changes);
    println!("{}", output);

    if !changes.is_empty() {
        info!("Changes have been saved to the database");
        info!("Review with: SELECT * FROM stop_pattern_changes WHERE acknowledged = FALSE;");
    }

    Ok(())
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

fn print_help() {
    println!("detect_stop_patterns - Detect train stop pattern changes from ODPT API");
    println!();
    println!("USAGE:");
    println!("    detect_stop_patterns [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --operators <LIST>  Comma-separated list of operators, or 'all'");
    println!("                            Default: TokyoMetro,Toei");
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
    println!();
    println!("To get an ODPT API key, register at: https://developer.odpt.org/");
}
