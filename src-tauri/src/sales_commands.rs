//! Point-of-sale commands.
//!
//! Classification: `get_pos_snapshot`, `open_pos_shift`, `complete_pos_sale`,
//! `get_sale_receipt`, `get_own_current_shift` and `get_own_current_shift_sales`
//! are `CASHIER_SCOPED`; `create_pos_refund` is `OWNER_OR_PERMISSION` with the
//! supervisory shift rule.

use localops_core::{
    cashier::{self, CashierSaleSummary, CashierSellableItem, OwnShiftView},
    payment, refund, sales, shift,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodDto {
    id: String,
    code: String,
    name: String,
    kind: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosSnapshot {
    business_name: String,
    currency: String,
    tax_enabled: bool,
    tax_rate_ppm: i64,
    prices_include_tax: bool,
    items: Vec<CashierSellableItem>,
    payment_methods: Vec<PaymentMethodDto>,
    open_shift: Option<OwnShiftView>,
    recent_sales: Vec<CashierSaleSummary>,
    can_discount: bool,
    can_refund: bool,
    can_void: bool,
    can_view_all_sales: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLineDto {
    id: String,
    description: String,
    kind: String,
    quantity_micros: i64,
    unit_price_minor: i64,
    discount_minor: i64,
    tax_minor: i64,
    line_total_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptPaymentDto {
    id: String,
    method_name: String,
    method_kind: String,
    amount_minor: i64,
    tendered_minor: Option<i64>,
    change_minor: i64,
    reference: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleReceiptDto {
    id: String,
    sale_number: String,
    currency: String,
    subtotal_minor: i64,
    discount_minor: i64,
    tax_minor: i64,
    total_minor: i64,
    amount_paid_minor: i64,
    change_due_minor: i64,
    completed_at: String,
    lines: Vec<ReceiptLineDto>,
    payments: Vec<ReceiptPaymentDto>,
    idempotent_replay: bool,
}

impl From<sales::CompletedSale> for SaleReceiptDto {
    fn from(value: sales::CompletedSale) -> Self {
        Self {
            id: value.id,
            sale_number: value.sale_number,
            currency: value.currency,
            subtotal_minor: value.subtotal_minor,
            discount_minor: value.discount_minor,
            tax_minor: value.tax_minor,
            total_minor: value.total_minor,
            amount_paid_minor: value.amount_paid_minor,
            change_due_minor: value.change_due_minor,
            completed_at: value.completed_at,
            lines: value
                .lines
                .into_iter()
                .map(|line| ReceiptLineDto {
                    id: line.id,
                    description: line.description,
                    kind: line.kind,
                    quantity_micros: line.quantity_micros,
                    unit_price_minor: line.unit_price_minor,
                    discount_minor: line.discount_minor,
                    tax_minor: line.tax_minor,
                    line_total_minor: line.line_total_minor,
                })
                .collect(),
            payments: value
                .payments
                .into_iter()
                .map(|item| ReceiptPaymentDto {
                    id: item.id,
                    method_name: item.method_name,
                    method_kind: item.method_kind,
                    amount_minor: item.amount_minor,
                    tendered_minor: item.tendered_minor,
                    change_minor: item.change_minor,
                    reference: item.reference,
                    status: item.status,
                })
                .collect(),
            idempotent_replay: value.idempotent_replay,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundedItemDto {
    sale_item_id: String,
    description: String,
    quantity_micros: i64,
    amount_minor: i64,
    restore_stock: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundedPaymentDto {
    payment_id: String,
    method_name: String,
    method_kind: String,
    amount_minor: i64,
    reference: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundReceiptDto {
    id: String,
    refund_number: String,
    sale_id: String,
    reason: String,
    total_minor: i64,
    sale_status: String,
    occurred_at: String,
    items: Vec<RefundedItemDto>,
    payments: Vec<RefundedPaymentDto>,
    idempotent_replay: bool,
}

impl From<refund::CompletedRefund> for RefundReceiptDto {
    fn from(value: refund::CompletedRefund) -> Self {
        Self {
            id: value.id,
            refund_number: value.refund_number,
            sale_id: value.sale_id,
            reason: value.reason,
            total_minor: value.total_minor,
            sale_status: value.sale_status,
            occurred_at: value.occurred_at,
            items: value
                .items
                .into_iter()
                .map(|item| RefundedItemDto {
                    sale_item_id: item.sale_item_id,
                    description: item.description,
                    quantity_micros: item.quantity_micros,
                    amount_minor: item.amount_minor,
                    restore_stock: item.restore_stock,
                })
                .collect(),
            payments: value
                .payments
                .into_iter()
                .map(|item| RefundedPaymentDto {
                    payment_id: item.payment_id,
                    method_name: item.method_name,
                    method_kind: item.method_kind,
                    amount_minor: item.amount_minor,
                    reference: item.reference,
                })
                .collect(),
            idempotent_replay: value.idempotent_replay,
        }
    }
}

/// `CASHIER_SCOPED`. Requires `pos.catalog.view`. The catalogue is limited to
/// the department of the authenticated session terminal, recent sales are
/// limited to the caller's own current open shift, and expected cash is only
/// present with `shifts.expected_cash.view`.
#[tauri::command]
pub fn get_pos_snapshot(state: State<'_, DbState>) -> Result<PosSnapshot, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("pos.catalog.view"))?;
    let (business_name, currency, tax_enabled, tax_rate_ppm, prices_include_tax): (
        String,
        String,
        bool,
        i64,
        bool,
    ) = connection
        .query_row(
            "SELECT name, currency, tax_enabled, tax_rate_ppm, prices_include_tax
             FROM businesses WHERE id = ?1",
            [&context.business_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;
    let methods = payment::ensure_default_methods(
        &connection,
        &context.business_id,
        &context.user_id,
        Some(&context.terminal_id),
    )
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|method| PaymentMethodDto {
        id: method.id,
        code: method.code,
        name: method.name,
        kind: method.kind,
    })
    .collect();
    let items = cashier::pos_catalogue(&connection, &context).map_err(|error| error.to_string())?;
    let open_shift = if context.has_permission("shifts.view_own_current") {
        cashier::get_own_current_shift(&connection, &context).map_err(|error| error.to_string())?
    } else {
        None
    };
    let recent_sales =
        cashier::recent_sales(&connection, &context, 25).map_err(|error| error.to_string())?;
    Ok(PosSnapshot {
        business_name,
        currency,
        tax_enabled,
        tax_rate_ppm,
        prices_include_tax,
        items,
        payment_methods: methods,
        open_shift,
        recent_sales,
        can_discount: context.has_permission("sales.discount"),
        can_refund: context.has_permission("sales.refund"),
        can_void: context.has_permission("sales.void"),
        can_view_all_sales: context.has_permission("sales.view_all"),
    })
}

/// `CASHIER_SCOPED`. Requires `shifts.view_own_current`.
#[tauri::command]
pub fn get_own_current_shift(state: State<'_, DbState>) -> Result<Option<OwnShiftView>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.view_own_current"))?;
    cashier::get_own_current_shift(&connection, &context).map_err(|error| error.to_string())
}

/// `CASHIER_SCOPED`. Requires `sales.view_own_current_shift` (or the broader
/// `sales.view_all` for Owner workflows).
#[tauri::command]
pub fn get_own_current_shift_sales(
    state: State<'_, DbState>,
) -> Result<Vec<CashierSaleSummary>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    cashier::recent_sales(&connection, &context, 50).map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenShiftInput {
    opening_balance_minor: i64,
    opened_at: String,
}

/// `CASHIER_SCOPED`. Requires `shifts.open_own`. Only the authenticated user
/// and the session terminal are used; an open shift owned by another employee
/// is reported as in use rather than handed over.
#[tauri::command]
pub fn open_pos_shift(
    state: State<'_, DbState>,
    input: OpenShiftInput,
) -> Result<OwnShiftView, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.open_own"))?;
    cashier::open_own_shift(
        &connection,
        &context,
        input.opening_balance_minor,
        &input.opened_at,
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleLineCommandInput {
    sellable_id: String,
    department_id: String,
    quantity_micros: i64,
    discount_minor: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalePaymentCommandInput {
    method_id: String,
    amount_minor: i64,
    tendered_minor: Option<i64>,
    reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSaleInput {
    idempotency_key: String,
    sale_number: Option<String>,
    completed_at: String,
    notes: Option<String>,
    lines: Vec<SaleLineCommandInput>,
    payments: Vec<SalePaymentCommandInput>,
}

/// `CASHIER_SCOPED`. Requires `sales.create` and `payments.record`, plus
/// `sales.discount` for any non-zero discount. The sale is bound to the
/// session business, terminal and department and to the caller's own open
/// shift; a frontend-supplied cashier, terminal or department is never trusted.
#[tauri::command]
pub fn complete_pos_sale(
    state: State<'_, DbState>,
    input: CompleteSaleInput,
) -> Result<SaleReceiptDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    let scopes = input
        .lines
        .iter()
        .map(|line| cashier::SaleLineScope {
            department_id: &line.department_id,
            discount_minor: line.discount_minor,
        })
        .collect::<Vec<_>>();
    cashier::authorize_sale_request(&context, &scopes).map_err(|error| error.to_string())?;
    let open_shift = localops_core::access::require_current_own_shift(&connection, &context)
        .map_err(|_| "Open your own shift before completing a sale".to_owned())?;
    let lines = input
        .lines
        .iter()
        .map(|line| sales::SaleLineInput {
            sellable_id: &line.sellable_id,
            department_id: &line.department_id,
            quantity_micros: line.quantity_micros,
            discount_minor: line.discount_minor,
        })
        .collect::<Vec<_>>();
    let payments = input
        .payments
        .iter()
        .map(|payment| sales::PaymentInput {
            method_id: &payment.method_id,
            amount_minor: payment.amount_minor,
            tendered_minor: payment.tendered_minor,
            reference: payment.reference.as_deref(),
        })
        .collect::<Vec<_>>();
    sales::complete_sale(
        &connection,
        sales::CompleteSale {
            business_id: &context.business_id,
            idempotency_key: &input.idempotency_key,
            sale_number: input.sale_number.as_deref(),
            terminal_id: &context.terminal_id,
            shift_id: &open_shift.id,
            cashier_id: &context.user_id,
            customer_id: None,
            completed_at: &input.completed_at,
            notes: input.notes.as_deref(),
            lines: &lines,
            payments: &payments,
        },
    )
    .map(Into::into)
    .map_err(|error| error.to_string())
}

/// `CASHIER_SCOPED` / `OWNER_OR_PERMISSION`.
///
/// Any later lookup by `sale_id` is a reprint. `receipts.reprint_all` allows
/// the Owner workflow; otherwise the caller needs
/// `receipts.reprint_own_current_shift` and the sale must be their own, on the
/// session terminal, inside their current open shift.
#[tauri::command]
pub fn get_sale_receipt(
    state: State<'_, DbState>,
    sale_id: String,
) -> Result<SaleReceiptDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    let scope = localops_core::access::authorize_receipt_reprint(&connection, &context, &sale_id)
        .map_err(|error| error.to_string())?;
    sales::load_completed_sale(&connection, &scope.sale_id)
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundItemCommandInput {
    sale_item_id: String,
    quantity_micros: i64,
    restore_stock: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundPaymentCommandInput {
    payment_id: String,
    amount_minor: i64,
    reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRefundInput {
    sale_id: String,
    idempotency_key: String,
    refund_number: Option<String>,
    reason: String,
    occurred_at: String,
    void_sale: bool,
    items: Vec<RefundItemCommandInput>,
    payments: Vec<RefundPaymentCommandInput>,
}

/// `OWNER_OR_PERMISSION`. Requires `sales.refund` or `sales.void`.
///
/// Supervisory rule: the reversal uses the session terminal's current open
/// shift even when that shift belongs to a Cashier. The action is attributed to
/// the authenticated actor and shift ownership is never changed.
#[tauri::command]
pub fn create_pos_refund(
    state: State<'_, DbState>,
    input: CreateRefundInput,
) -> Result<RefundReceiptDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let permission = if input.void_sale {
        "sales.void"
    } else {
        "sales.refund"
    };
    let context = require_active_context(&connection, &state, Some(permission))?;
    let open_shift = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Open a shift before processing a reversal".to_owned())?;
    let items = input
        .items
        .iter()
        .map(|item| refund::RefundItemInput {
            sale_item_id: &item.sale_item_id,
            quantity_micros: item.quantity_micros,
            restore_stock: item.restore_stock,
        })
        .collect::<Vec<_>>();
    let payments = input
        .payments
        .iter()
        .map(|payment| refund::RefundPaymentInput {
            payment_id: &payment.payment_id,
            amount_minor: payment.amount_minor,
            reference: payment.reference.as_deref(),
        })
        .collect::<Vec<_>>();
    refund::create_refund(
        &connection,
        refund::CreateRefund {
            business_id: &context.business_id,
            sale_id: &input.sale_id,
            idempotency_key: &input.idempotency_key,
            refund_number: input.refund_number.as_deref(),
            shift_id: &open_shift.id,
            terminal_id: &context.terminal_id,
            user_id: &context.user_id,
            reason: &input.reason,
            occurred_at: &input.occurred_at,
            void_sale: input.void_sale,
            items: &items,
            payments: &payments,
        },
    )
    .map(Into::into)
    .map_err(|error| error.to_string())
}
