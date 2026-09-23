use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

pub const DEFAULT_INACTIVITY_MINUTES: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub business_id: String,
    pub user_id: String,
    pub terminal_id: String,
    pub started_at: String,
    pub last_active_at: String,
    pub ended_at: Option<String>,
    pub end_reason: Option<String>,
}

pub fn start_session(
    connection: &Connection,
    business_id: &str,
    user_id: &str,
    terminal_id: &str,
) -> Result<Session> {
    let id = Uuid::now_v7().to_string();
    let changed = connection.execute(
        "INSERT INTO sessions(
             id, business_id, user_id, terminal_id, started_at, last_active_at
         )
         SELECT ?1, ?2, u.id, t.id,
                strftime('%Y-%m-%dT%H:%M:%fZ','now'),
                strftime('%Y-%m-%dT%H:%M:%fZ','now')
         FROM users u
         JOIN terminals t ON t.business_id = u.business_id
         WHERE u.id = ?3 AND t.id = ?4 AND u.business_id = ?2
               AND u.active = 1 AND t.active = 1",
        (&id, business_id, user_id, terminal_id),
    )?;
    if changed == 0 {
        return Err(CoreError::CrossBusinessReference);
    }
    get_session(connection, &id)?.ok_or(CoreError::SessionNotFound)
}

pub fn get_session(connection: &Connection, id: &str) -> Result<Option<Session>> {
    connection
        .query_row(
            "SELECT id, business_id, user_id, terminal_id, started_at,
                    last_active_at, ended_at, end_reason
             FROM sessions WHERE id = ?1",
            [id],
            map_session,
        )
        .optional()
        .map_err(Into::into)
}

pub fn resume_session(connection: &Connection, id: &str) -> Result<Option<Session>> {
    let active = connection
        .query_row(
            "SELECT id, business_id, user_id, terminal_id, started_at,
                    last_active_at, ended_at, end_reason
             FROM sessions
             WHERE id = ?1 AND ended_at IS NULL
               AND julianday(last_active_at) >= julianday('now', ?2)",
            (id, format!("-{DEFAULT_INACTIVITY_MINUTES} minutes")),
            map_session,
        )
        .optional()?;
    if active.is_some() {
        return Ok(active);
    }
    connection.execute(
        "UPDATE sessions
         SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), end_reason = 'INACTIVITY'
         WHERE id = ?1 AND ended_at IS NULL",
        [id],
    )?;
    Ok(None)
}

pub fn touch_session(connection: &Connection, id: &str) -> Result<()> {
    if resume_session(connection, id)?.is_none() {
        return Err(CoreError::SessionNotFound);
    }
    connection.execute(
        "UPDATE sessions
         SET last_active_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1 AND ended_at IS NULL",
        [id],
    )?;
    Ok(())
}

pub fn end_session(connection: &Connection, id: &str, reason: &str) -> Result<()> {
    let changed = connection.execute(
        "UPDATE sessions
         SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), end_reason = ?2
         WHERE id = ?1 AND ended_at IS NULL",
        (id, reason),
    )?;
    if changed == 0 {
        return Err(CoreError::SessionNotFound);
    }
    Ok(())
}

fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        business_id: row.get(1)?,
        user_id: row.get(2)?,
        terminal_id: row.get(3)?,
        started_at: row.get(4)?,
        last_active_at: row.get(5)?,
        ended_at: row.get(6)?,
        end_reason: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database, terminal, user};

    #[test]
    fn starts_touches_and_ends_local_session() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let terminal_id =
            terminal::create_terminal(&database, &business_id, None, "Till", Some("T1")).unwrap();
        let user_id = user::create_user(&database, &business_id, "owner", "Owner", "1234").unwrap();
        let session = start_session(&database, &business_id, &user_id, &terminal_id).unwrap();
        touch_session(&database, &session.id).unwrap();
        assert!(resume_session(&database, &session.id).unwrap().is_some());
        end_session(&database, &session.id, "LOGOUT").unwrap();
        assert!(resume_session(&database, &session.id).unwrap().is_none());
    }

    #[test]
    fn blocks_cross_business_terminal_session() {
        let database = open_memory_database().unwrap();
        let business_a = business::create_business(&database, "Business A").unwrap();
        let business_b = business::create_business(&database, "Business B").unwrap();
        let user_id = user::create_user(&database, &business_a, "owner", "Owner", "1234").unwrap();
        let terminal_id =
            terminal::create_terminal(&database, &business_b, None, "Till", None).unwrap();
        assert!(matches!(
            start_session(&database, &business_a, &user_id, &terminal_id),
            Err(CoreError::CrossBusinessReference)
        ));
    }

    #[test]
    fn expires_inactive_session() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let terminal_id =
            terminal::create_terminal(&database, &business_id, None, "Till", None).unwrap();
        let user_id = user::create_user(&database, &business_id, "owner", "Owner", "1234").unwrap();
        let session = start_session(&database, &business_id, &user_id, &terminal_id).unwrap();
        database
            .execute(
                "UPDATE sessions SET last_active_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
                [&session.id],
            )
            .unwrap();
        assert!(resume_session(&database, &session.id).unwrap().is_none());
        assert_eq!(
            get_session(&database, &session.id)
                .unwrap()
                .unwrap()
                .end_reason
                .as_deref(),
            Some("INACTIVITY")
        );
    }
}
