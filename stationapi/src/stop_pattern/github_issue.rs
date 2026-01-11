//! GitHub Issue Creator
//!
//! Creates GitHub issues when stop pattern changes are detected.

use super::detector::StopPatternChange;
use serde::Serialize;
use tracing::{error, info};

/// GitHub Issue creator for stop pattern changes
pub struct GitHubIssueCreator {
    client: reqwest::Client,
    token: String,
    repo: String, // format: "owner/repo"
}

#[derive(Serialize)]
struct CreateIssueRequest {
    title: String,
    body: String,
    labels: Vec<String>,
}

impl GitHubIssueCreator {
    /// Create a new GitHubIssueCreator
    ///
    /// # Arguments
    /// * `token` - GitHub personal access token with `repo` scope
    /// * `repo` - Repository in "owner/repo" format
    pub fn new(token: String, repo: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            token,
            repo,
        }
    }

    /// Create an issue for detected changes
    pub async fn create_issue(
        &self,
        changes: &[StopPatternChange],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if changes.is_empty() {
            return Ok("No changes to report".to_string());
        }

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let title = format!("[自動検出] 停車パターン変更 {}", today);

        let body = self.format_issue_body(changes);

        let request = CreateIssueRequest {
            title,
            body,
            labels: vec!["stop-pattern-change".to_string(), "automated".to_string()],
        };

        let url = format!("https://api.github.com/repos/{}/issues", self.repo);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "stationapi-stop-pattern-detector")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let issue: serde_json::Value = response.json().await?;
            let issue_url = issue["html_url"].as_str().unwrap_or("unknown");
            info!("Created GitHub issue: {}", issue_url);
            Ok(issue_url.to_string())
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            error!("Failed to create issue: {} - {}", status, error_body);
            Err(format!("GitHub API error: {} - {}", status, error_body).into())
        }
    }

    fn format_issue_body(&self, changes: &[StopPatternChange]) -> String {
        let mut body = String::new();

        body.push_str("## 停車パターン変更が検出されました\n\n");
        body.push_str(&format!("検出された変更: **{}件**\n\n", changes.len()));

        // Group by railway
        let mut grouped: std::collections::HashMap<(&str, &str), Vec<&StopPatternChange>> =
            std::collections::HashMap::new();

        for change in changes {
            let key = (change.railway_id.as_str(), change.train_type_id.as_str());
            grouped.entry(key).or_default().push(change);
        }

        for ((railway_id, _), changes) in grouped {
            let first = changes.first().unwrap();
            body.push_str(&format!("### {} ({})\n", first.railway_name, railway_id));
            body.push_str(&format!("種別: {}\n\n", first.train_type_name));

            let added: Vec<_> = changes
                .iter()
                .filter(|c| c.change_type == super::detector::ChangeType::Added)
                .collect();
            let removed: Vec<_> = changes
                .iter()
                .filter(|c| c.change_type == super::detector::ChangeType::Removed)
                .collect();

            if !added.is_empty() {
                body.push_str("**新規停車:**\n");
                for change in added {
                    body.push_str(&format!(
                        "- {} (`{}`)\n",
                        change.station_name, change.station_id
                    ));
                }
                body.push('\n');
            }

            if !removed.is_empty() {
                body.push_str("**停車取りやめ:**\n");
                for change in removed {
                    body.push_str(&format!(
                        "- {} (`{}`)\n",
                        change.station_name, change.station_id
                    ));
                }
                body.push('\n');
            }
        }

        body.push_str("---\n");
        body.push_str("このIssueは自動作成されました。\n");
        body.push_str("変更を確認したら、DBで `acknowledged = TRUE` に更新してください。\n\n");
        body.push_str("```sql\n");
        body.push_str(
            "UPDATE stop_pattern_changes SET acknowledged = TRUE, acknowledged_at = NOW()\n",
        );
        body.push_str("WHERE acknowledged = FALSE;\n");
        body.push_str("```\n");

        body
    }
}
