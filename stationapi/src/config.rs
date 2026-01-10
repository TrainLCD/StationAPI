use std::env;

/// Fetch the DATABASE_URL environment variable.
/// Panics if the variable is not set or is not valid Unicode.
pub fn fetch_database_url() -> String {
    match env::var("DATABASE_URL") {
        Ok(s) => s,
        Err(env::VarError::NotPresent) => panic!("$DATABASE_URL is not set."),
        Err(env::VarError::NotUnicode(_)) => panic!("$DATABASE_URL should be written in Unicode."),
    }
}

/// Fetch the ODPT_API_KEY environment variable.
/// Returns None if the variable is not set.
pub fn fetch_odpt_api_key() -> Option<String> {
    env::var("ODPT_API_KEY").ok()
}

/// Check if rail GTFS feature is enabled via ENABLE_RAIL_GTFS environment variable.
/// Defaults to true if ODPT_API_KEY is set, false otherwise.
pub fn is_rail_gtfs_enabled() -> bool {
    match env::var("ENABLE_RAIL_GTFS") {
        Ok(s) => s.to_lowercase() == "true" || s == "1",
        Err(_) => fetch_odpt_api_key().is_some(),
    }
}

/// Check if a specific GTFS source is enabled via environment variable.
/// Example: GTFS_ENABLE_TOKYO_METRO=true
pub fn is_gtfs_source_enabled(source_id: &str) -> bool {
    let env_key = format!("GTFS_ENABLE_{}", source_id.to_uppercase().replace('-', "_"));
    match env::var(&env_key) {
        Ok(s) => s.to_lowercase() == "true" || s == "1",
        Err(_) => true, // Default to enabled
    }
}
