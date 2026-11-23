use chrono::{DateTime, Utc};
use libsql::{params, Database};

pub struct TursoService {
    db: Database,
}

/// Pure helper determining whether an errored recipe is still blocked
/// (either retries exhausted or cooldown period has not elapsed).
pub fn is_error_blocked(
    retry_count: i64,
    last_attempt_str: &str,
    max_retries: i64,
    cooldown_hours: i64,
    now: DateTime<Utc>,
) -> bool {
    if retry_count >= max_retries {
        return true;
    }

    if let Ok(last_attempt) = DateTime::parse_from_rfc3339(last_attempt_str) {
        let duration = now.signed_duration_since(last_attempt.with_timezone(&Utc));
        if duration.num_hours() < cooldown_hours {
            return true;
        }
    }

    false
}

impl TursoService {
    pub async fn new(url: &str, token: &str) -> Result<Self, libsql::Error> {
        let db = libsql::Builder::new_remote(url.to_string(), token.to_string())
            .build()
            .await?;

        let conn = db.connect()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audited_recipes (
                recipe_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                last_attempt TEXT NOT NULL,
                error_msg TEXT
            )",
            (),
        )
        .await?;

        Ok(Self { db })
    }

    /// Returns recipe IDs currently in `error` status that cannot be retried yet
    /// (retries exhausted or within cooldown), so MongoDB can skip them in `$nin`.
    pub async fn get_blocked_error_recipe_ids(
        &self,
        max_retries: i64,
        cooldown_hours: i64,
    ) -> Result<Vec<String>, libsql::Error> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT recipe_id, retry_count, last_attempt FROM audited_recipes WHERE status = 'error'",
                (),
            )
            .await?;

        let now = Utc::now();
        let mut blocked = Vec::new();
        while let Some(row) = rows.next().await? {
            let recipe_id: String = row.get(0)?;
            let retry_count: i64 = row.get(1)?;
            let last_attempt_str: String = row.get(2)?;

            if is_error_blocked(
                retry_count,
                &last_attempt_str,
                max_retries,
                cooldown_hours,
                now,
            ) {
                blocked.push(recipe_id);
            }
        }

        Ok(blocked)
    }

    /// Determines if a recipe needs auditing or a retry.
    /// Returns true if it has never been processed, OR if it errored and is ready for retry.
    pub async fn needs_processing(
        &self,
        recipe_id: &str,
        max_retries: i64,
        cooldown_hours: i64,
    ) -> Result<bool, libsql::Error> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT status, retry_count, last_attempt FROM audited_recipes WHERE recipe_id = ?1",
                params![recipe_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let status: String = row.get(0)?;
            let retry_count: i64 = row.get(1)?;
            let last_attempt_str: String = row.get(2)?;

            if status == "ok" || status == "skipped" {
                return Ok(false);
            }

            if status == "error" {
                return Ok(!is_error_blocked(
                    retry_count,
                    &last_attempt_str,
                    max_retries,
                    cooldown_hours,
                    Utc::now(),
                ));
            }

            Ok(false)
        } else {
            Ok(true) // Never processed
        }
    }

    pub async fn mark_ok(&self, recipe_id: &str) -> Result<(), libsql::Error> {
        let conn = self.db.connect()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audited_recipes (recipe_id, status, retry_count, last_attempt, error_msg)
             VALUES (?1, 'ok', 0, ?2, NULL)
             ON CONFLICT(recipe_id) DO UPDATE SET
                status = 'ok',
                retry_count = 0,
                last_attempt = ?2,
                error_msg = NULL",
            params![recipe_id, now],
        )
        .await?;
        Ok(())
    }

    pub async fn mark_skipped(&self, recipe_id: &str) -> Result<(), libsql::Error> {
        let conn = self.db.connect()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO audited_recipes (recipe_id, status, retry_count, last_attempt, error_msg)
             VALUES (?1, 'skipped', 0, ?2, NULL)
             ON CONFLICT(recipe_id) DO UPDATE SET
                status = 'skipped',
                retry_count = 0,
                last_attempt = ?2,
                error_msg = NULL",
            params![recipe_id, now],
        )
        .await?;
        Ok(())
    }

    pub async fn mark_error(&self, recipe_id: &str, error_msg: &str) -> Result<(), libsql::Error> {
        let conn = self.db.connect()?;
        let now = Utc::now().to_rfc3339();

        // Get current retry count
        let mut rows = conn
            .query(
                "SELECT retry_count FROM audited_recipes WHERE recipe_id = ?1",
                params![recipe_id],
            )
            .await?;

        let new_retry_count = if let Some(row) = rows.next().await? {
            let current_count: i64 = row.get(0)?;
            current_count + 1
        } else {
            1
        };

        conn.execute(
            "INSERT INTO audited_recipes (recipe_id, status, retry_count, last_attempt, error_msg)
             VALUES (?1, 'error', ?2, ?3, ?4)
             ON CONFLICT(recipe_id) DO UPDATE SET
                status = 'error',
                retry_count = ?2,
                last_attempt = ?3,
                error_msg = ?4",
            params![recipe_id, new_retry_count, now, error_msg],
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_is_error_blocked_exhausted_retries() {
        let now = Utc::now();
        let old_attempt = (now - Duration::hours(48)).to_rfc3339();
        assert!(is_error_blocked(3, &old_attempt, 3, 24, now));
        assert!(is_error_blocked(4, &old_attempt, 3, 24, now));
    }

    #[test]
    fn test_is_error_blocked_within_cooldown() {
        let now = Utc::now();
        let recent_attempt = (now - Duration::hours(5)).to_rfc3339();
        assert!(is_error_blocked(1, &recent_attempt, 3, 24, now));
    }

    #[test]
    fn test_is_error_blocked_ready_after_cooldown() {
        let now = Utc::now();
        let expired_attempt = (now - Duration::hours(25)).to_rfc3339();
        assert!(!is_error_blocked(1, &expired_attempt, 3, 24, now));
        assert!(!is_error_blocked(2, &expired_attempt, 3, 24, now));
    }
}
