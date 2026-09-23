use rusqlite::Connection;
use uuid::Uuid;

use crate::Result;

#[derive(Debug, Clone, Copy)]
pub struct NewAuditEvent<'a> {
    pub business_id: &'a str,
    pub user_id: Option<&'a str>,
    pub terminal_id: Option<&'a str>,
    pub action: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub old_json: Option<&'a str>,
    pub new_json: Option<&'a str>,
}

pub fn record_event(connection: &Connection, event: NewAuditEvent<'_>) -> Result<String> {
    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO audit_logs(
             id, business_id, user_id, terminal_id, action, entity_type,
             entity_id, occurred_at, old_json, new_json, correlation_id
         ) VALUES(
             ?1, ?2, ?3, ?4, ?5, ?6, ?7,
             strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?8, ?9, ?10
         )",
        (
            &id,
            event.business_id,
            event.user_id,
            event.terminal_id,
            event.action,
            event.entity_type,
            event.entity_id,
            event.old_json,
            event.new_json,
            Uuid::now_v7().to_string(),
        ),
    )?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database};

    #[test]
    fn records_structured_business_audit_event() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test").unwrap();
        let id = record_event(
            &database,
            NewAuditEvent {
                business_id: &business_id,
                user_id: None,
                terminal_id: None,
                action: "TESTED",
                entity_type: "business",
                entity_id: &business_id,
                old_json: None,
                new_json: Some(r#"{"ready":true}"#),
            },
        )
        .unwrap();
        let stored: (String, String) = database
            .query_row(
                "SELECT action, new_json FROM audit_logs WHERE id = ?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            stored,
            ("TESTED".to_owned(), r#"{"ready":true}"#.to_owned())
        );
    }
}
