//! Shift management and expense commands.
//!
//! Classification: `get_operations_snapshot`, `close_any_shift`,
//! `create_expense_category`, `record_operating_expense` and
//! `void_operating_expense` are `OWNER_OR_PERMISSION`; `close_own_shift` is
//! `CASHIER_SCOPED`.

use localops_core::{cashier, department, expense, payment, shift};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftView {
    id: String,
    terminal_id: String,
    opening_balance_minor: i64,
    expected_balance_minor: Option<i64>,
    actual_balance_minor: Option<i64>,
    variance_minor: Option<i64>,
    status: String,
    opened_at: String,
    closed_at: Option<String>,
    opened_by_name: Option<String>,
    closed_by_name: Option<String>,
    notes: Option<String>,
}

/// Build the management shift view, filtering the reconciliation fields by the
/// caller's explicit permissions.
fn shift_view(
    connection: &rusqlite::Connection,
    context: &localops_core::access::AccessContext,
    value: shift::Shift,
) -> ShiftView {
    let name_of = |user_id: Option<&str>| -> Option<String> {
        user_id.and_then(|id| {
            connection
                .query_row("SELECT display_name FROM users WHERE id = ?1", [id], |row| {
                    row.get::<_, String>(0)
                })
                .ok()
        })
    };
    ShiftView {
        opened_by_name: name_of(Some(value.user_id.as_str())),
        closed_by_name: name_of(value.closed_by.as_deref()),
        id: value.id,
        terminal_id: value.terminal_id,
        opening_balance_minor: value.opening_balance_minor,
        expected_balance_minor: context
            .has_permission("shifts.expected_cash.view")
            .then_some(value.expected_balance_minor),
        actual_balance_minor: value.actual_balance_minor,
        variance_minor: context
            .has_permission("shifts.variance.view")
            .then_some(value.variance_minor)
            .flatten(),
        status: value.status,
        opened_at: value.opened_at,
        closed_at: value.closed_at,
        notes: value.notes,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionView {
    id: String,
    name: String,
    kind: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseView {
    id: String,
    category_name: String,
    department_name: Option<String>,
    payment_method_name: Option<String>,
    amount_minor: i64,
    currency: String,
    expense_date: String,
    description: String,
    reference: Option<String>,
    receipt_image_path: Option<String>,
    status: String,
    shift_id: String,
}

impl From<expense::Expense> for ExpenseView {
    fn from(value: expense::Expense) -> Self {
        Self {
            id: value.id,
            category_name: value.category_name,
            department_name: value.department_name,
            payment_method_name: value.payment_method_name,
            amount_minor: value.amount_minor,
            currency: value.currency,
            expense_date: value.expense_date,
            description: value.description,
            reference: value.reference,
            receipt_image_path: value.receipt_image_path,
            status: value.status,
            shift_id: value.shift_id,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationsSnapshot {
    open_shift: Option<ShiftView>,
    shifts: Vec<ShiftView>,
    categories: Vec<OptionView>,
    departments: Vec<OptionView>,
    payment_methods: Vec<OptionView>,
    expenses: Vec<ExpenseView>,
}

/// `OWNER_OR_PERMISSION`. Requires `shifts.view_all`; expected cash, variance
/// and expenses are filtered by their own explicit permissions.
#[tauri::command]
pub fn get_operations_snapshot(state: State<'_, DbState>) -> Result<OperationsSnapshot, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.view_all"))?;
    let open_shift = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .map(|value| shift_view(&connection, &context, value));
    let shifts = shift::list_shifts(&connection, &context.business_id, 30)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| shift_view(&connection, &context, value))
        .collect();
    let categories = expense::list_categories(&connection, &context.business_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| OptionView {
            id: value.id,
            name: value.name,
            kind: None,
        })
        .collect();
    let departments = department::list_departments(&connection, &context.business_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| OptionView {
            id: value.id,
            name: value.name,
            kind: None,
        })
        .collect();
    let payment_methods = payment::ensure_default_methods(
        &connection,
        &context.business_id,
        &context.user_id,
        Some(&context.terminal_id),
    )
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|value| OptionView {
        id: value.id,
        name: value.name,
        kind: Some(value.kind),
    })
    .collect();
    let expenses = if context.has_permission("expenses.view") {
        expense::list_expenses(&connection, &context.business_id, 50)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(Into::into)
            .collect()
    } else {
        Vec::new()
    };
    Ok(OperationsSnapshot {
        open_shift,
        shifts,
        categories,
        departments,
        payment_methods,
        expenses,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseShiftInput {
    actual_balance_minor: i64,
    closed_at: String,
    notes: Option<String>,
}

/// `CASHIER_SCOPED`. Requires `shifts.close_own`.
///
/// Blind close: the Cashier submits a counted amount, the backend persists
/// expected cash and variance, and the response withholds both unless the
/// caller separately holds the management permissions for those fields.
#[tauri::command]
pub fn close_own_shift(
    state: State<'_, DbState>,
    input: CloseShiftInput,
) -> Result<cashier::ShiftCloseResult, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.close_own"))?;
    cashier::close_own_shift(
        &connection,
        &context,
        input.actual_balance_minor,
        &input.closed_at,
        input.notes.as_deref(),
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseAnyShiftInput {
    shift_id: String,
    actual_balance_minor: i64,
    closed_at: String,
    notes: Option<String>,
}

/// `OWNER_OR_PERMISSION`. Requires `shifts.close_any`.
///
/// Supervisory close of an abandoned or unfinished shift. The original owner is
/// preserved and `closed_by` records the authenticated supervisor.
#[tauri::command]
pub fn close_any_shift(
    state: State<'_, DbState>,
    input: CloseAnyShiftInput,
) -> Result<cashier::ShiftCloseResult, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.close_any"))?;
    cashier::close_any_shift(
        &connection,
        &context,
        &input.shift_id,
        input.actual_balance_minor,
        &input.closed_at,
        input.notes.as_deref(),
    )
    .map_err(|error| error.to_string())
}

/// `OWNER_OR_PERMISSION` (`expenses.manage`).
#[tauri::command]
pub fn create_expense_category(state: State<'_, DbState>, name: String) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("expenses.manage"))?;
    expense::create_category(
        &connection,
        &context.business_id,
        &name,
        &context.user_id,
        Some(&context.terminal_id),
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseInput {
    department_id: Option<String>,
    category_id: String,
    payment_method_id: Option<String>,
    amount_minor: i64,
    expense_date: String,
    description: String,
    reference: Option<String>,
    receipt_image_path: Option<String>,
    idempotency_key: String,
}

#[tauri::command]
pub fn record_operating_expense(
    state: State<'_, DbState>,
    input: ExpenseInput,
) -> Result<ExpenseView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("expenses.manage"))?;
    let opened = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Open a shift before recording an expense".to_owned())?;
    expense::record_expense(
        &connection,
        expense::NewExpense {
            business_id: &context.business_id,
            department_id: input.department_id.as_deref(),
            category_id: &input.category_id,
            payment_method_id: input.payment_method_id.as_deref(),
            shift_id: &opened.id,
            terminal_id: &context.terminal_id,
            amount_minor: input.amount_minor,
            expense_date: &input.expense_date,
            description: &input.description,
            reference: input.reference.as_deref(),
            receipt_image_path: input.receipt_image_path.as_deref(),
            idempotency_key: &input.idempotency_key,
            user_id: &context.user_id,
        },
    )
    .map(Into::into)
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidExpenseInput {
    expense_id: String,
    occurred_at: String,
    reason: String,
}

#[tauri::command]
pub fn void_operating_expense(
    state: State<'_, DbState>,
    input: VoidExpenseInput,
) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("expenses.manage"))?;
    expense::void_expense(
        &connection,
        &context.business_id,
        &input.expense_id,
        &context.user_id,
        &context.terminal_id,
        &input.occurred_at,
        &input.reason,
    )
    .map_err(|error| error.to_string())
}
