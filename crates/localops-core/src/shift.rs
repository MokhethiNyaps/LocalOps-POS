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
    pub opened_at: String,
}

pub fn get_open_shift(
    connection: &Connection,
    business_id: &str,
    terminal_id: &str,
) -> Result<Option<Shift>> {
    connection
        .query_row(
            "SELECT id, business_id, terminal_id, user_id, opening_balance_minor, opened_at
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
                    opened_at: row.get(5)?,
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
        opened_at: opened_at.to_owned(),
    })
}
