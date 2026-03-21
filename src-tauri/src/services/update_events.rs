use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::adapters::UpdateDecision;
use crate::error::{AppError, AppResult};
use crate::models::UpdateStatus;

#[derive(Debug)]
pub struct UpdateEvents;

impl UpdateEvents {
    /// Creates an update event and returns the event ID.
    pub fn create_event(
        conn: &Connection,
        local_mod_id: &str,
        binding_id: Option<&str>,
        decision: &UpdateDecision,
        latest_version: Option<&str>,
        latest_published_at: Option<&str>,
    ) -> AppResult<String> {
        if local_mod_id.is_empty() {
            return Err(AppError::Message("local_mod_id cannot be empty".into()));
        }

        tracing::info!(
            "Creating update event for mod {} with status {:?}",
            local_mod_id,
            decision.status
        );

        let event_id = Uuid::new_v4().to_string();
        let event_type = match decision.status {
            UpdateStatus::ConfirmedUpdate => "confirmed_update",
            UpdateStatus::ProbableUpdate => "probable_update",
            UpdateStatus::SourceActivity => "source_activity",
            UpdateStatus::SourceUnreachable => "source_unreachable",
            UpdateStatus::UpToDate => "up_to_date",
            UpdateStatus::Untracked => "untracked",
        };
        let confidence_score = decision.confidence;
        let summary = decision
            .summary
            .clone()
            .unwrap_or_else(|| event_type.to_owned());
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO update_events (id, local_mod_id, binding_id, event_type, confidence_score, summary, latest_version_text, latest_published_at, is_read, is_dismissed, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, 0, ?9)",
            params![
                event_id,
                local_mod_id,
                binding_id,
                event_type,
                confidence_score,
                summary,
                latest_version,
                latest_published_at,
                created_at
            ],
        )?;

        Ok(event_id)
    }

    /// Returns unread, undismissed events, ordered by creation time descending.
    pub fn get_unread_events(conn: &Connection, limit: i64) -> AppResult<Vec<UpdateEventRow>> {
        if limit < 0 {
            return Err(AppError::Message("limit cannot be negative".into()));
        }

        let mut stmt = conn.prepare(
            "SELECT id, local_mod_id, binding_id, event_type, confidence_score, summary,
                    latest_version_text, latest_published_at, is_read, is_dismissed, created_at
             FROM update_events
             WHERE is_read = 0 AND is_dismissed = 0
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(UpdateEventRow {
                id: row.get(0)?,
                local_mod_id: row.get(1)?,
                binding_id: row.get(2)?,
                event_type: row.get(3)?,
                confidence_score: row.get(4)?,
                summary: row.get(5)?,
                latest_version_text: row.get(6)?,
                latest_published_at: row.get(7)?,
                is_read: row.get::<_, i64>(8)? != 0,
                is_dismissed: row.get::<_, i64>(9)? != 0,
                created_at: row.get(10)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        tracing::debug!("Retrieved {} unread events", events.len());
        Ok(events)
    }

    /// Marks an event as read.
    pub fn mark_read(conn: &Connection, event_id: &str) -> AppResult<()> {
        if event_id.is_empty() {
            return Err(AppError::Message("event_id cannot be empty".into()));
        }

        conn.execute(
            "UPDATE update_events SET is_read = 1 WHERE id = ?1",
            params![event_id],
        )?;
        tracing::debug!("Marked event {} as read", event_id);
        Ok(())
    }

    /// Dismisses an event.
    pub fn dismiss_event(conn: &Connection, event_id: &str) -> AppResult<()> {
        if event_id.is_empty() {
            return Err(AppError::Message("event_id cannot be empty".into()));
        }

        conn.execute(
            "UPDATE update_events SET is_dismissed = 1 WHERE id = ?1",
            params![event_id],
        )?;
        tracing::debug!("Dismissed event {}", event_id);
        Ok(())
    }

    /// Returns counts of update events by type.
    pub fn get_update_counts(conn: &Connection) -> AppResult<UpdateCounts> {
        let mut stmt = conn.prepare(
            "SELECT event_type, COUNT(*) as count
             FROM update_events
             WHERE is_dismissed = 0
             GROUP BY event_type",
        )?;

        let mut confirmed_updates = 0i64;
        let mut probable_updates = 0i64;
        let mut source_activity = 0i64;

        let rows = stmt.query_map([], |row| {
            let event_type: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((event_type, count))
        })?;

        for row in rows {
            let (event_type, count) = row?;
            match event_type.as_str() {
                "confirmed_update" => confirmed_updates = count,
                "probable_update" => probable_updates = count,
                "source_activity" => source_activity = count,
                _ => {}
            }
        }

        tracing::debug!(
            "Update counts: confirmed={}, probable={}, activity={}",
            confirmed_updates,
            probable_updates,
            source_activity
        );

        Ok(UpdateCounts {
            confirmed_updates,
            probable_updates,
            source_activity,
            total: confirmed_updates + probable_updates + source_activity,
        })
    }

    /// Returns events for a specific mod, ordered by creation time descending.
    pub fn get_events_for_mod(
        conn: &Connection,
        local_mod_id: &str,
        limit: i64,
    ) -> AppResult<Vec<UpdateEventRow>> {
        if local_mod_id.is_empty() {
            return Err(AppError::Message("local_mod_id cannot be empty".into()));
        }
        if limit < 0 {
            return Err(AppError::Message("limit cannot be negative".into()));
        }

        let mut stmt = conn.prepare(
            "SELECT id, local_mod_id, binding_id, event_type, confidence_score, summary,
                    latest_version_text, latest_published_at, is_read, is_dismissed, created_at
             FROM update_events
             WHERE local_mod_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![local_mod_id, limit], |row| {
            Ok(UpdateEventRow {
                id: row.get(0)?,
                local_mod_id: row.get(1)?,
                binding_id: row.get(2)?,
                event_type: row.get(3)?,
                confidence_score: row.get(4)?,
                summary: row.get(5)?,
                latest_version_text: row.get(6)?,
                latest_published_at: row.get(7)?,
                is_read: row.get::<_, i64>(8)? != 0,
                is_dismissed: row.get::<_, i64>(9)? != 0,
                created_at: row.get(10)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        tracing::debug!("Retrieved {} events for mod {}", events.len(), local_mod_id);
        Ok(events)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEventRow {
    pub id: String,
    pub local_mod_id: String,
    pub binding_id: Option<String>,
    pub event_type: String,
    pub confidence_score: f64,
    pub summary: String,
    pub latest_version_text: Option<String>,
    pub latest_published_at: Option<String>,
    pub is_read: bool,
    pub is_dismissed: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCounts {
    pub confirmed_updates: i64,
    pub probable_updates: i64,
    pub source_activity: i64,
    pub total: i64,
}
