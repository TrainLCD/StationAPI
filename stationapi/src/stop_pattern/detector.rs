//! Stop Pattern Detector
//!
//! Detects changes in train stop patterns by comparing current data with stored snapshots.

use super::odpt_client::{OdptClient, OdptOperator, StopPattern};
use sqlx::PgPool;
use std::collections::HashSet;
use tracing::info;

/// Represents a detected change in stop pattern
#[derive(Debug, Clone)]
pub struct StopPatternChange {
    pub operator_id: String,
    pub railway_id: String,
    pub railway_name: String,
    pub train_type_id: String,
    pub train_type_name: String,
    pub change_type: ChangeType,
    pub station_id: String,
    pub station_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Removed,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeType::Added => "added",
            ChangeType::Removed => "removed",
        }
    }
}

/// Previous snapshot from database
#[derive(Debug, Clone)]
struct StoredSnapshot {
    pub railway_id: String,
    pub train_type_id: String,
    pub station_ids: Vec<String>,
}

/// Stop Pattern Detector
pub struct StopPatternDetector {
    client: OdptClient,
    pool: PgPool,
}

impl StopPatternDetector {
    pub fn new(api_key: String, pool: PgPool) -> Self {
        Self {
            client: OdptClient::new(api_key),
            pool,
        }
    }

    /// Run detection for specified operators
    pub async fn detect_changes(
        &self,
        operators: &[OdptOperator],
    ) -> Result<Vec<StopPatternChange>, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Starting stop pattern detection for {} operators",
            operators.len()
        );

        // Fetch current patterns from ODPT API
        let current_patterns = self.client.extract_all_stop_patterns(operators).await?;

        info!("Fetched {} current patterns", current_patterns.len());

        // Get previous snapshots from database
        let previous_snapshots = self.get_latest_snapshots().await?;
        info!("Loaded {} previous snapshots", previous_snapshots.len());

        // Detect changes
        let changes = self.compare_patterns(&current_patterns, &previous_snapshots);

        if !changes.is_empty() {
            info!("Detected {} changes", changes.len());

            // Save changes to database
            self.save_changes(&changes).await?;
        } else {
            info!("No changes detected");
        }

        // Save current patterns as new snapshots
        self.save_snapshots(&current_patterns).await?;

        Ok(changes)
    }

    /// Get the latest snapshot for each railway/train_type combination
    async fn get_latest_snapshots(
        &self,
    ) -> Result<Vec<StoredSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query_as::<_, (String, String, Vec<String>)>(
            r#"
            SELECT DISTINCT ON (railway_id, train_type_id)
                railway_id, train_type_id, station_ids
            FROM stop_pattern_snapshots
            ORDER BY railway_id, train_type_id, captured_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(railway_id, train_type_id, station_ids)| StoredSnapshot {
                railway_id,
                train_type_id,
                station_ids,
            })
            .collect())
    }

    /// Compare current patterns with previous snapshots
    fn compare_patterns(
        &self,
        current: &[StopPattern],
        previous: &[StoredSnapshot],
    ) -> Vec<StopPatternChange> {
        let mut changes = Vec::new();

        // Build lookup map for previous snapshots
        let prev_map: std::collections::HashMap<(&str, &str), &StoredSnapshot> = previous
            .iter()
            .map(|s| ((s.railway_id.as_str(), s.train_type_id.as_str()), s))
            .collect();

        for pattern in current {
            let key = (pattern.railway_id.as_str(), pattern.train_type_id.as_str());

            let current_stations: HashSet<&str> =
                pattern.station_ids.iter().map(|s| s.as_str()).collect();

            if let Some(prev) = prev_map.get(&key) {
                let prev_stations: HashSet<&str> =
                    prev.station_ids.iter().map(|s| s.as_str()).collect();

                // Find added stations
                for station_id in current_stations.difference(&prev_stations) {
                    let station_name = pattern
                        .station_ids
                        .iter()
                        .zip(pattern.station_names.iter())
                        .find(|(id, _)| id.as_str() == *station_id)
                        .map(|(_, name)| name.clone())
                        .unwrap_or_else(|| station_id.to_string());

                    changes.push(StopPatternChange {
                        operator_id: pattern.operator_id.clone(),
                        railway_id: pattern.railway_id.clone(),
                        railway_name: pattern.railway_name.clone(),
                        train_type_id: pattern.train_type_id.clone(),
                        train_type_name: pattern.train_type_name.clone(),
                        change_type: ChangeType::Added,
                        station_id: station_id.to_string(),
                        station_name,
                    });
                }

                // Find removed stations
                for station_id in prev_stations.difference(&current_stations) {
                    changes.push(StopPatternChange {
                        operator_id: pattern.operator_id.clone(),
                        railway_id: pattern.railway_id.clone(),
                        railway_name: pattern.railway_name.clone(),
                        train_type_id: pattern.train_type_id.clone(),
                        train_type_name: pattern.train_type_name.clone(),
                        change_type: ChangeType::Removed,
                        station_id: station_id.to_string(),
                        station_name: station_id.to_string(), // Name not available for removed
                    });
                }
            }
            // Note: We don't report "new" railway/train_type combinations as changes
            // since that would generate too much noise on first run
        }

        changes
    }

    /// Save detected changes to database
    async fn save_changes(
        &self,
        changes: &[StopPatternChange],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for change in changes {
            sqlx::query(
                r#"
                INSERT INTO stop_pattern_changes
                    (operator_id, railway_id, railway_name, train_type_id, train_type_name,
                     change_type, station_id, station_name)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(&change.operator_id)
            .bind(&change.railway_id)
            .bind(&change.railway_name)
            .bind(&change.train_type_id)
            .bind(&change.train_type_name)
            .bind(change.change_type.as_str())
            .bind(&change.station_id)
            .bind(&change.station_name)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Save current patterns as snapshots
    async fn save_snapshots(
        &self,
        patterns: &[StopPattern],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for pattern in patterns {
            sqlx::query(
                r#"
                INSERT INTO stop_pattern_snapshots
                    (operator_id, railway_id, train_type_id, train_type_name, station_ids, station_names, captured_date)
                VALUES ($1, $2, $3, $4, $5, $6, CURRENT_DATE)
                ON CONFLICT (railway_id, train_type_id, captured_date)
                DO UPDATE SET
                    station_ids = EXCLUDED.station_ids,
                    station_names = EXCLUDED.station_names,
                    train_type_name = EXCLUDED.train_type_name
                "#,
            )
            .bind(&pattern.operator_id)
            .bind(&pattern.railway_id)
            .bind(&pattern.train_type_id)
            .bind(&pattern.train_type_name)
            .bind(&pattern.station_ids)
            .bind(&pattern.station_names)
            .execute(&self.pool)
            .await?;
        }

        info!("Saved {} snapshots", patterns.len());
        Ok(())
    }

    /// Get unacknowledged changes
    pub async fn get_unacknowledged_changes(
        &self,
    ) -> Result<Vec<StopPatternChange>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT operator_id, railway_id, railway_name, train_type_id, train_type_name,
                   change_type, station_id, station_name
            FROM stop_pattern_changes
            WHERE acknowledged = FALSE
            ORDER BY detected_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    operator_id,
                    railway_id,
                    railway_name,
                    train_type_id,
                    train_type_name,
                    change_type,
                    station_id,
                    station_name,
                )| {
                    StopPatternChange {
                        operator_id,
                        railway_id,
                        railway_name,
                        train_type_id,
                        train_type_name,
                        change_type: if change_type == "added" {
                            ChangeType::Added
                        } else {
                            ChangeType::Removed
                        },
                        station_id,
                        station_name,
                    }
                },
            )
            .collect())
    }

    /// Format changes for display
    pub fn format_changes(changes: &[StopPatternChange]) -> String {
        if changes.is_empty() {
            return "変更は検出されませんでした。".to_string();
        }

        let mut output = String::new();
        output.push_str(&format!("検出された変更: {} 件\n\n", changes.len()));

        // Group by railway and train type
        let mut grouped: std::collections::HashMap<(&str, &str), Vec<&StopPatternChange>> =
            std::collections::HashMap::new();

        for change in changes {
            let key = (change.railway_id.as_str(), change.train_type_id.as_str());
            grouped.entry(key).or_default().push(change);
        }

        for ((railway_id, _train_type_id), changes) in grouped {
            let first = changes.first().unwrap();
            output.push_str(&format!("路線: {} ({})\n", first.railway_name, railway_id));
            output.push_str(&format!("種別: {}\n", first.train_type_name));
            output.push('\n');

            let added: Vec<_> = changes
                .iter()
                .filter(|c| c.change_type == ChangeType::Added)
                .collect();
            let removed: Vec<_> = changes
                .iter()
                .filter(|c| c.change_type == ChangeType::Removed)
                .collect();

            if !added.is_empty() {
                output.push_str("新規停車:\n");
                for change in added {
                    output.push_str(&format!(
                        "  + {} ({})\n",
                        change.station_name, change.station_id
                    ));
                }
            }

            if !removed.is_empty() {
                output.push_str("停車取りやめ:\n");
                for change in removed {
                    output.push_str(&format!(
                        "  - {} ({})\n",
                        change.station_name, change.station_id
                    ));
                }
            }

            output.push_str("---\n");
        }

        output
    }
}
