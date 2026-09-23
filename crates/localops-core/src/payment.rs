use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::{CoreError, Result, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentMethod {
    pub id: String,
    pub code: String,
    pub name: String,
    pub kind: String,
}

pub fn ensure_default_methods(
    connection: &Connection,
    business_id: &str,
    user_id: &str,
    terminal_id: Option<&str>,
) -> Result<Vec<PaymentMethod>> {
    let existing = list_methods(connection, business_id)?;
    if !existing.is_empty() {
        return Ok(existing);
    }
    let transaction = connection.unchecked_transaction()?;
    let valid_user = transaction
        .query_row(
            "SELECT 1 FROM users WHERE id = ?1 AND business_id = ?2 AND active = 1",
            (user_id, business_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !valid_user {
        return Err(CoreError::CrossBusinessReference);
    }
    for (code, name, kind) in [
        ("CASH", "Cash", "CASH"),
        ("CARD", "Card", "CARD"),
        ("EFT", "EFT", "EFT"),
        ("OTHER", "Other", "OTHER"),
    ] {
        transaction.execute(
            "INSERT INTO payment_methods(id, business_id, code, name, kind)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            (Uuid::now_v7().to_string(), business_id, code, name, kind),
        )?;
    }
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id,
            user_id: Some(user_id),
            terminal_id,
            action: "DEFAULT_PAYMENT_METHODS_CREATED",
            entity_type: "business",
            entity_id: business_id,
            old_json: None,
            new_json: Some(r#"{"codes":["CASH","CARD","EFT","OTHER"]}"#),
        },
    )?;
    transaction.commit()?;
    list_methods(connection, business_id)
}

pub fn list_methods(connection: &Connection, business_id: &str) -> Result<Vec<PaymentMethod>> {
    let mut statement = connection.prepare(
        "SELECT id, code, name, kind FROM payment_methods
         WHERE business_id = ?1 AND active = 1 ORDER BY name",
    )?;
    statement
        .query_map([business_id], |row| {
            Ok(PaymentMethod {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                kind: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}
