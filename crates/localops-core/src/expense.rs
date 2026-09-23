use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{CoreError, Result, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpenseCategory {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct NewExpense<'a> {
    pub business_id: &'a str,
    pub department_id: Option<&'a str>,
    pub category_id: &'a str,
    pub payment_method_id: Option<&'a str>,
    pub shift_id: &'a str,
    pub terminal_id: &'a str,
    pub amount_minor: i64,
    pub expense_date: &'a str,
    pub description: &'a str,
    pub reference: Option<&'a str>,
    pub receipt_image_path: Option<&'a str>,
    pub idempotency_key: &'a str,
    pub user_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expense {
    pub id: String,
    pub category_name: String,
    pub department_name: Option<String>,
    pub payment_method_name: Option<String>,
    pub amount_minor: i64,
    pub currency: String,
    pub expense_date: String,
    pub description: String,
    pub reference: Option<String>,
    pub receipt_image_path: Option<String>,
    pub status: String,
    pub shift_id: String,
    pub idempotent_replay: bool,
}

pub fn create_category(
    connection: &Connection,
    business_id: &str,
    name: &str,
    user_id: &str,
    terminal_id: Option<&str>,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyExpenseCategoryName);
    }
    let transaction = connection.unchecked_transaction()?;
    let valid: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?2 AND business_id = ?1 AND active = 1)",
        (business_id, user_id),
        |row| row.get(0),
    )?;
    if !valid {
        return Err(CoreError::CrossBusinessReference);
    }
    let id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO expense_categories(id, business_id, name) VALUES(?1, ?2, ?3)",
        (&id, business_id, name),
    )?;
    let payload = format!("{{\"name\":\"{name}\"}}");
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id,
            user_id: Some(user_id),
            terminal_id,
            action: "EXPENSE_CATEGORY_CREATED",
            entity_type: "expense_category",
            entity_id: &id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(id)
}

pub fn list_categories(connection: &Connection, business_id: &str) -> Result<Vec<ExpenseCategory>> {
    let mut statement = connection.prepare(
        "SELECT id, name FROM expense_categories
         WHERE business_id = ?1 AND active = 1 ORDER BY name",
    )?;
    statement
        .query_map([business_id], |row| {
            Ok(ExpenseCategory {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn record_expense(connection: &Connection, input: NewExpense<'_>) -> Result<Expense> {
    if let Some(existing_id) = connection
        .query_row(
            "SELECT id FROM expenses WHERE business_id = ?1 AND idempotency_key = ?2",
            (input.business_id, input.idempotency_key),
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        let mut existing = get_expense(connection, input.business_id, &existing_id)?;
        existing.idempotent_replay = true;
        return Ok(existing);
    }
    if input.amount_minor <= 0 {
        return Err(CoreError::InvalidExpenseAmount);
    }
    if input.description.trim().is_empty() {
        return Err(CoreError::EmptyExpenseDescription);
    }
    let transaction = connection.unchecked_transaction()?;
    let (currency, category_name): (String, String) = transaction
        .query_row(
            "SELECT b.currency, ec.name
             FROM businesses b JOIN expense_categories ec ON ec.business_id = b.id
             WHERE b.id = ?1 AND b.active = 1 AND ec.id = ?2 AND ec.active = 1",
            (input.business_id, input.category_id),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(CoreError::ExpenseCategoryNotFound)?;
    let shift_valid: bool = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM shifts sh
             JOIN terminals t ON t.id = sh.terminal_id AND t.business_id = sh.business_id
             JOIN users u ON u.id = sh.user_id AND u.business_id = sh.business_id
             WHERE sh.id = ?2 AND sh.business_id = ?1 AND sh.terminal_id = ?3
               AND sh.user_id = ?4 AND sh.status = 'OPEN' AND t.active = 1 AND u.active = 1
         )",
        (
            input.business_id,
            input.shift_id,
            input.terminal_id,
            input.user_id,
        ),
        |row| row.get(0),
    )?;
    if !shift_valid {
        return Err(CoreError::OpenShiftNotFound);
    }
    if let Some(department_id) = input.department_id {
        let valid: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM departments WHERE id = ?2 AND business_id = ?1 AND active = 1)",
            (input.business_id, department_id),
            |row| row.get(0),
        )?;
        if !valid {
            return Err(CoreError::CrossBusinessReference);
        }
    }
    let payment = input
        .payment_method_id
        .map(|payment_id| {
            transaction
                .query_row(
                    "SELECT name, kind FROM payment_methods
                     WHERE id = ?2 AND business_id = ?1 AND active = 1",
                    (input.business_id, payment_id),
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
        })
        .transpose()?
        .flatten();
    if input.payment_method_id.is_some() && payment.is_none() {
        return Err(CoreError::PaymentMethodNotFound);
    }
    if payment.as_ref().is_some_and(|value| value.1 == "CASH") {
        let updated = transaction.execute(
            "UPDATE shifts SET expected_balance_minor = expected_balance_minor - ?2
             WHERE id = ?1 AND status = 'OPEN' AND expected_balance_minor >= ?2",
            (input.shift_id, input.amount_minor),
        )?;
        if updated != 1 {
            return Err(CoreError::InvalidExpenseAmount);
        }
    }
    let id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO expenses(
             id, business_id, department_id, category_id, payment_method_id,
             amount_minor, currency, expense_date, description, reference,
             receipt_image_path, created_by, shift_id, terminal_id, idempotency_key
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            &id,
            input.business_id,
            input.department_id,
            input.category_id,
            input.payment_method_id,
            input.amount_minor,
            &currency,
            input.expense_date,
            input.description.trim(),
            input.reference,
            input.receipt_image_path,
            input.user_id,
            input.shift_id,
            input.terminal_id,
            input.idempotency_key,
        ],
    )?;
    let payload = format!(
        "{{\"amountMinor\":{},\"categoryId\":\"{}\"}}",
        input.amount_minor, input.category_id
    );
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id: input.business_id,
            user_id: Some(input.user_id),
            terminal_id: Some(input.terminal_id),
            action: "EXPENSE_RECORDED",
            entity_type: "expense",
            entity_id: &id,
            old_json: None,
            new_json: Some(&payload),
        },
    )?;
    transaction.commit()?;
    Ok(Expense {
        id,
        category_name,
        department_name: None,
        payment_method_name: payment.map(|value| value.0),
        amount_minor: input.amount_minor,
        currency,
        expense_date: input.expense_date.to_owned(),
        description: input.description.trim().to_owned(),
        reference: input.reference.map(str::to_owned),
        receipt_image_path: input.receipt_image_path.map(str::to_owned),
        status: "RECORDED".to_owned(),
        shift_id: input.shift_id.to_owned(),
        idempotent_replay: false,
    })
}

pub fn void_expense(
    connection: &Connection,
    business_id: &str,
    expense_id: &str,
    user_id: &str,
    terminal_id: &str,
    occurred_at: &str,
    reason: &str,
) -> Result<()> {
    if reason.trim().is_empty() {
        return Err(CoreError::EmptyExpenseDescription);
    }
    let transaction = connection.unchecked_transaction()?;
    let (status, amount_minor, shift_id, payment_kind): (String, i64, String, Option<String>) =
        transaction
            .query_row(
                "SELECT e.status, e.amount_minor, e.shift_id, pm.kind
                 FROM expenses e LEFT JOIN payment_methods pm ON pm.id = e.payment_method_id
                 JOIN users u ON u.id = ?3 AND u.business_id = e.business_id AND u.active = 1
                 WHERE e.id = ?2 AND e.business_id = ?1",
                (business_id, expense_id, user_id),
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or(CoreError::ExpenseNotFound)?;
    if status == "VOID" {
        return Err(CoreError::ExpenseAlreadyVoided);
    }
    if payment_kind.as_deref() == Some("CASH") {
        let updated = transaction.execute(
            "UPDATE shifts SET expected_balance_minor = expected_balance_minor + ?2
             WHERE id = ?1 AND status = 'OPEN'",
            (&shift_id, amount_minor),
        )?;
        if updated != 1 {
            return Err(CoreError::ShiftNotOpen);
        }
    }
    transaction.execute(
        "UPDATE expenses SET status = 'VOID', voided_at = ?2, void_reason = ?3,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1 AND status = 'RECORDED'",
        (expense_id, occurred_at, reason.trim()),
    )?;
    audit::record_event(
        &transaction,
        audit::NewAuditEvent {
            business_id,
            user_id: Some(user_id),
            terminal_id: Some(terminal_id),
            action: "EXPENSE_VOIDED",
            entity_type: "expense",
            entity_id: expense_id,
            old_json: None,
            new_json: None,
        },
    )?;
    transaction.commit()?;
    Ok(())
}

pub fn get_expense(
    connection: &Connection,
    business_id: &str,
    expense_id: &str,
) -> Result<Expense> {
    connection
        .query_row(
            "SELECT e.id, ec.name, d.name, pm.name, e.amount_minor, e.currency,
                    e.expense_date, e.description, e.reference, e.receipt_image_path,
                    e.status, e.shift_id
             FROM expenses e
             JOIN expense_categories ec ON ec.id = e.category_id
             LEFT JOIN departments d ON d.id = e.department_id
             LEFT JOIN payment_methods pm ON pm.id = e.payment_method_id
             WHERE e.id = ?2 AND e.business_id = ?1",
            (business_id, expense_id),
            |row| {
                Ok(Expense {
                    id: row.get(0)?,
                    category_name: row.get(1)?,
                    department_name: row.get(2)?,
                    payment_method_name: row.get(3)?,
                    amount_minor: row.get(4)?,
                    currency: row.get(5)?,
                    expense_date: row.get(6)?,
                    description: row.get(7)?,
                    reference: row.get(8)?,
                    receipt_image_path: row.get(9)?,
                    status: row.get(10)?,
                    shift_id: row.get(11)?,
                    idempotent_replay: false,
                })
            },
        )
        .optional()?
        .ok_or(CoreError::ExpenseNotFound)
}

pub fn list_expenses(
    connection: &Connection,
    business_id: &str,
    limit: i64,
) -> Result<Vec<Expense>> {
    let mut statement = connection.prepare(
        "SELECT e.id, ec.name, d.name, pm.name, e.amount_minor, e.currency,
                e.expense_date, e.description, e.reference, e.receipt_image_path,
                e.status, e.shift_id
         FROM expenses e
         JOIN expense_categories ec ON ec.id = e.category_id
         LEFT JOIN departments d ON d.id = e.department_id
         LEFT JOIN payment_methods pm ON pm.id = e.payment_method_id
         WHERE e.business_id = ?1 ORDER BY e.expense_date DESC, e.created_at DESC LIMIT ?2",
    )?;
    statement
        .query_map((business_id, limit), |row| {
            Ok(Expense {
                id: row.get(0)?,
                category_name: row.get(1)?,
                department_name: row.get(2)?,
                payment_method_name: row.get(3)?,
                amount_minor: row.get(4)?,
                currency: row.get(5)?,
                expense_date: row.get(6)?,
                description: row.get(7)?,
                reference: row.get(8)?,
                receipt_image_path: row.get(9)?,
                status: row.get(10)?,
                shift_id: row.get(11)?,
                idempotent_replay: false,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{payment, setup, shift};

    #[test]
    fn cash_expense_is_idempotent_audited_and_reversible_before_close() {
        let database = crate::open_memory_database().unwrap();
        let setup = setup::complete_initial_setup(
            &database,
            setup::InitialSetup {
                business_name: "Expense Business",
                department_name: "Main",
                location_name: "Store",
                terminal_name: "Till",
                owner_username: "owner",
                owner_display_name: "Owner",
                owner_pin: "1234",
            },
        )
        .unwrap();
        let cash = payment::ensure_default_methods(
            &database,
            &setup.business_id,
            &setup.user_id,
            Some(&setup.terminal_id),
        )
        .unwrap()
        .into_iter()
        .find(|method| method.kind == "CASH")
        .unwrap();
        let opened = shift::open_shift(
            &database,
            &setup.business_id,
            &setup.terminal_id,
            &setup.user_id,
            1_000,
            "2026-09-23T08:00:00.000Z",
        )
        .unwrap();
        let category_id = create_category(
            &database,
            &setup.business_id,
            "Fuel",
            &setup.user_id,
            Some(&setup.terminal_id),
        )
        .unwrap();
        let input = NewExpense {
            business_id: &setup.business_id,
            department_id: Some(&setup.department_id),
            category_id: &category_id,
            payment_method_id: Some(&cash.id),
            shift_id: &opened.id,
            terminal_id: &setup.terminal_id,
            amount_minor: 500,
            expense_date: "2026-09-23",
            description: "Generator fuel",
            reference: Some("SLIP-1"),
            receipt_image_path: None,
            idempotency_key: "expense-key-1",
            user_id: &setup.user_id,
        };
        let expense = record_expense(&database, input).unwrap();
        assert_eq!(expense.amount_minor, 500);
        let replay = record_expense(&database, input).unwrap();
        assert_eq!(replay.id, expense.id);
        assert!(replay.idempotent_replay);
        assert_eq!(get_expected(&database, &opened.id), 500);
        void_expense(
            &database,
            &setup.business_id,
            &expense.id,
            &setup.user_id,
            &setup.terminal_id,
            "2026-09-23T10:00:00.000Z",
            "Duplicate slip",
        )
        .unwrap();
        assert_eq!(get_expected(&database, &opened.id), 1_000);
        assert_eq!(
            get_expense(&database, &setup.business_id, &expense.id)
                .unwrap()
                .status,
            "VOID"
        );
        assert!(matches!(
            void_expense(
                &database,
                &setup.business_id,
                &expense.id,
                &setup.user_id,
                &setup.terminal_id,
                "2026-09-23T10:01:00.000Z",
                "Again",
            ),
            Err(CoreError::ExpenseAlreadyVoided)
        ));
    }

    fn get_expected(database: &Connection, shift_id: &str) -> i64 {
        database
            .query_row(
                "SELECT expected_balance_minor FROM shifts WHERE id = ?1",
                [shift_id],
                |row| row.get(0),
            )
            .unwrap()
    }
}
