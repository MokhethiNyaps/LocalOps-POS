use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::{CoreError, Result, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shift {
    pub id: String,
    pub business_id: String,
    pub terminal_id: String,
    pub user_id: String,
    pub opening_balance_minor: i64,
    pub expected_balance_minor: i64,
    pub actual_balance_minor: Option<i64>,
    pub variance_minor: Option<i64>,
    pub status: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub notes: Option<String>,
}

pub fn get_open_shift(
    connection: &Connection,
    business_id: &str,
    terminal_id: &str,
) -> Result<Option<Shift>> {
    connection
        .query_row(
            "SELECT id, business_id, terminal_id, user_id, opening_balance_minor,
                    expected_balance_minor, actual_balance_minor, variance_minor,
                    status, opened_at, closed_at, notes
             FROM shifts WHERE business_id = ?1 AND terminal_id = ?2 AND status = 'OPEN'
             ORDER BY opened_at DESC LIMIT 1",
            (business_id, terminal_id),
            |row| {
                Ok(Shift {
                    id: row.get(0)?,
                    business_id: row.get(1)?,
                    terminal_id: row.get(2)?,
                    user_id: row.get(3)?,
                    opening_balance_minor: row.get(4)?,
                    expected_balance_minor: row.get(5)?,
                    actual_balance_minor: row.get(6)?,
                    variance_minor: row.get(7)?,
                    status: row.get(8)?,
                    opened_at: row.get(9)?,
                    closed_at: row.get(10)?,
                    notes: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub fn open_shift(
    connection: &Connection,
    business_id: &str,
    terminal_id: &str,
    user_id: &str,
    opening_balance_minor: i64,
    opened_at: &str,
) -> Result<Shift> {
    if let Some(existing) = get_open_shift(connection, business_id, terminal_id)? {
        return Ok(existing);
    }
    if opening_balance_minor < 0 {
        return Err(CoreError::MoneyOverflow);
    }
    let transaction = connection.unchecked_transaction()?;
    let valid = transaction
        .query_row(
            "SELECT 1 FROM terminals t
             JOIN users u ON u.business_id = t.business_id
             WHERE t.id = ?2 AND t.business_id = ?1 AND t.active = 1
               AND u.id = ?3 AND u.active = 1",
            (business_id, terminal_id, user_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !valid {
        return Err(CoreError::CrossBusinessReference);
    }
    let id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO shifts(
             id, business_id, terminal_id, user_id, opening_balance_minor,
             expected_balance_minor, opened_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?5, ?6)",
        (
            &id,
            business_id,
            terminal_id,
            user_id,
            opening_balance_minor,
            opened_at,
        ),
    )?;
    let payload = format!("{{\"openingBalanceMinor\":{opening_balance_minor}}}");
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id,
            user_id: Some(user_id),
            terminal_id: Some(terminal_id),
            action: "SHIFT_OPENED",
            entity_type: "shift",
            entity_id: &id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(Shift {
        id,
        business_id: business_id.to_owned(),
        terminal_id: terminal_id.to_owned(),
        user_id: user_id.to_owned(),
        opening_balance_minor,
        expected_balance_minor: opening_balance_minor,
        actual_balance_minor: None,
        variance_minor: None,
        status: "OPEN".to_owned(),
        opened_at: opened_at.to_owned(),
        closed_at: None,
        notes: None,
    })
}

pub fn close_shift(
    connection: &Connection,
    business_id: &str,
    shift_id: &str,
    closed_by: &str,
    actual_balance_minor: i64,
    closed_at: &str,
    notes: Option<&str>,
) -> Result<Shift> {
    if actual_balance_minor < 0 {
        return Err(CoreError::InvalidActualBalance);
    }
    let transaction = connection.unchecked_transaction()?;
    let mut current = transaction
        .query_row(
            "SELECT sh.id, sh.business_id, sh.terminal_id, sh.user_id,
                    sh.opening_balance_minor, sh.expected_balance_minor,
                    sh.actual_balance_minor, sh.variance_minor, sh.status,
                    sh.opened_at, sh.closed_at, sh.notes
             FROM shifts sh
             JOIN users u ON u.id = ?3 AND u.business_id = sh.business_id AND u.active = 1
             WHERE sh.id = ?2 AND sh.business_id = ?1",
            (business_id, shift_id, closed_by),
            |row| {
                Ok(Shift {
                    id: row.get(0)?,
                    business_id: row.get(1)?,
                    terminal_id: row.get(2)?,
                    user_id: row.get(3)?,
                    opening_balance_minor: row.get(4)?,
                    expected_balance_minor: row.get(5)?,
                    actual_balance_minor: row.get(6)?,
                    variance_minor: row.get(7)?,
                    status: row.get(8)?,
                    opened_at: row.get(9)?,
                    closed_at: row.get(10)?,
                    notes: row.get(11)?,
                })
            },
        )
        .optional()?
        .ok_or(CoreError::ShiftNotOpen)?;
    if current.status != "OPEN" {
        return Err(CoreError::ShiftNotOpen);
    }
    let variance_minor = actual_balance_minor
        .checked_sub(current.expected_balance_minor)
        .ok_or(CoreError::MoneyOverflow)?;
    transaction.execute(
        "UPDATE shifts SET status = 'CLOSED', actual_balance_minor = ?2,
             variance_minor = ?3, closed_at = ?4, closed_by = ?5, notes = ?6
         WHERE id = ?1 AND status = 'OPEN'",
        (
            shift_id,
            actual_balance_minor,
            variance_minor,
            closed_at,
            closed_by,
            notes,
        ),
    )?;
    let payload = format!(
        "{{\"expectedBalanceMinor\":{},\"actualBalanceMinor\":{},\"varianceMinor\":{}}}",
        current.expected_balance_minor, actual_balance_minor, variance_minor
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id,
            user_id: Some(closed_by),
            terminal_id: Some(&current.terminal_id),
            action: "SHIFT_CLOSED",
            entity_type: "shift",
            entity_id: shift_id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    current.actual_balance_minor = Some(actual_balance_minor);
    current.variance_minor = Some(variance_minor);
    current.status = "CLOSED".to_owned();
    current.closed_at = Some(closed_at.to_owned());
    current.notes = notes.map(str::to_owned);
    Ok(current)
}

pub fn list_shifts(connection: &Connection, business_id: &str, limit: i64) -> Result<Vec<Shift>> {
    let mut statement = connection.prepare(
        "SELECT id, business_id, terminal_id, user_id, opening_balance_minor,
                expected_balance_minor, actual_balance_minor, variance_minor,
                status, opened_at, closed_at, notes
         FROM shifts WHERE business_id = ?1
         ORDER BY opened_at DESC, created_at DESC LIMIT ?2",
    )?;
    statement
        .query_map((business_id, limit), |row| {
            Ok(Shift {
                id: row.get(0)?,
                business_id: row.get(1)?,
                terminal_id: row.get(2)?,
                user_id: row.get(3)?,
                opening_balance_minor: row.get(4)?,
                expected_balance_minor: row.get(5)?,
                actual_balance_minor: row.get(6)?,
                variance_minor: row.get(7)?,
                status: row.get(8)?,
                opened_at: row.get(9)?,
                closed_at: row.get(10)?,
                notes: row.get(11)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup;

    #[test]
    fn closes_shift_with_immutable_expected_actual_and_variance() {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Shift Business",
                department_name: "Main",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let opened = open_shift(
            &database,
            &setup.business_id,
            &setup.terminal_id,
            &setup.user_id,
            1_000,
            "2026-09-23T08:00:00.000Z",
        )
        .unwrap();
        let closed = close_shift(
            &database,
            &setup.business_id,
            &opened.id,
            &setup.user_id,
            900,
            "2026-09-23T17:00:00.000Z",
            Some("Counted by owner"),
        )
        .unwrap();
        assert_eq!(closed.status, "CLOSED");
        assert_eq!(closed.expected_balance_minor, 1_000);
        assert_eq!(closed.actual_balance_minor, Some(900));
        assert_eq!(closed.variance_minor, Some(-100));
        assert!(
            get_open_shift(&database, &setup.business_id, &setup.terminal_id)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            close_shift(
                &database,
                &setup.business_id,
                &opened.id,
                &setup.user_id,
                900,
                "2026-09-23T17:01:00.000Z",
                None,
            ),
            Err(CoreError::ShiftNotOpen)
        ));
    }
}
