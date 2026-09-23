use localops_core::{department, expense, payment, shift};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftView {
    id: String,
    terminal_id: String,
    opening_balance_minor: i64,
    expected_balance_minor: i64,
    actual_balance_minor: Option<i64>,
    variance_minor: Option<i64>,
    status: String,
    opened_at: String,
    closed_at: Option<String>,
    notes: Option<String>,
}

impl From<shift::Shift> for ShiftView {
    fn from(value: shift::Shift) -> Self {
        Self {
            id: value.id,
            terminal_id: value.terminal_id,
            opening_balance_minor: value.opening_balance_minor,
            expected_balance_minor: value.expected_balance_minor,
            actual_balance_minor: value.actual_balance_minor,
            variance_minor: value.variance_minor,
            status: value.status,
            opened_at: value.opened_at,
            closed_at: value.closed_at,
            notes: value.notes,
        }
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

#[tauri::command]
pub fn get_operations_snapshot(state: State<'_, DbState>) -> Result<OperationsSnapshot, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    let open_shift = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .map(Into::into);
    let shifts = shift::list_shifts(&connection, &context.business_id, 30)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(Into::into)
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
    let expenses = expense::list_expenses(&connection, &context.business_id, 50)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(Into::into)
        .collect();
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

#[tauri::command]
pub fn close_current_shift(
    state: State<'_, DbState>,
    input: CloseShiftInput,
) -> Result<ShiftView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.manage"))?;
    let opened = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No open shift for this terminal".to_owned())?;
    shift::close_shift(
        &connection,
        &context.business_id,
        &opened.id,
        &context.user_id,
        input.actual_balance_minor,
        &input.closed_at,
        input.notes.as_deref(),
    )
    .map(Into::into)
    .map_err(|error| error.to_string())
}

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
