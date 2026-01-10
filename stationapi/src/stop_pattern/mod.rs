//! Stop Pattern Detection Module
//!
//! This module provides functionality to detect changes in train stop patterns
//! by fetching data from ODPT (Open Data for Public Transportation) API
//! and comparing it with previously stored snapshots.

pub mod detector;
pub mod odpt_client;

pub use detector::StopPatternDetector;
pub use odpt_client::OdptClient;
