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
