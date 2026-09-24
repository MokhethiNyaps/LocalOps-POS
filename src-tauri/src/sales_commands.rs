use localops_core::{payment, refund, role, sales, shift};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

const POS_ITEM_QUERY: &str = "SELECT s.id, d.id, d.name, s.name, s.kind,
            COALESCE(ds.price_override_minor, s.price_minor), s.taxable,
            s.sku, s.product_code, s.barcode
     FROM department_sellables ds
     JOIN departments d ON d.id = ds.department_id
     JOIN sellable_items s ON s.id = ds.sellable_id
     WHERE d.business_id = ?1 AND d.active = 1 AND s.active = 1 AND ds.active = 1
     ORDER BY d.name, s.name";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosItemDto {
    id: String,
    department_id: String,
    department_name: String,
    name: String,
    kind: String,
    price_minor: i64,
    taxable: bool,
    sku: Option<String>,
    product_code: Option<String>,
    barcode: Option<String>,
}

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
pub struct ShiftDto {
    id: String,
    opening_balance_minor: i64,
    expected_balance_minor: i64,
    opened_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentSaleDto {
    id: String,
    sale_number: String,
    status: String,
    total_minor: i64,
    completed_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosSnapshot {
    business_name: String,
    currency: String,
    tax_enabled: bool,
    tax_rate_ppm: i64,
    prices_include_tax: bool,
    items: Vec<PosItemDto>,
    payment_methods: Vec<PaymentMethodDto>,
    open_shift: Option<ShiftDto>,
    recent_sales: Vec<RecentSaleDto>,
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

#[tauri::command]
pub fn get_pos_snapshot(state: State<'_, DbState>) -> Result<PosSnapshot, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
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
    let mut item_statement = connection
        .prepare(POS_ITEM_QUERY)
        .map_err(|error| error.to_string())?;
    let items = item_statement
        .query_map([&context.business_id], |row| {
            Ok(PosItemDto {
                id: row.get(0)?,
                department_id: row.get(1)?,
                department_name: row.get(2)?,
                name: row.get(3)?,
                kind: row.get(4)?,
                price_minor: row.get(5)?,
                taxable: row.get(6)?,
                sku: row.get(7)?,
                product_code: row.get(8)?,
                barcode: row.get(9)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let open_shift = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .map(|value| {
            let expected_balance_minor = connection
                .query_row(
                    "SELECT expected_balance_minor FROM shifts WHERE id = ?1",
                    [&value.id],
                    |row| row.get(0),
                )
                .unwrap_or(value.opening_balance_minor);
            ShiftDto {
                id: value.id,
                opening_balance_minor: value.opening_balance_minor,
                expected_balance_minor,
                opened_at: value.opened_at,
            }
        });
    let mut sale_statement = connection
        .prepare(
            "SELECT id, sale_number, status, total_minor, completed_at
             FROM sales WHERE business_id = ?1
             ORDER BY completed_at DESC, created_at DESC LIMIT 25",
        )
        .map_err(|error| error.to_string())?;
    let recent_sales = sale_statement
        .query_map([&context.business_id], |row| {
            Ok(RecentSaleDto {
                id: row.get(0)?,
                sale_number: row.get(1)?,
                status: row.get(2)?,
                total_minor: row.get(3)?,
                completed_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
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
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenShiftInput {
    opening_balance_minor: i64,
    opened_at: String,
}

#[tauri::command]
pub fn open_pos_shift(state: State<'_, DbState>, input: OpenShiftInput) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("shifts.manage"))?;
    shift::open_shift(
        &connection,
        &context.business_id,
        &context.terminal_id,
        &context.user_id,
        input.opening_balance_minor,
        &input.opened_at,
    )
    .map(|value| value.id)
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

#[tauri::command]
pub fn complete_pos_sale(
    state: State<'_, DbState>,
    input: CompleteSaleInput,
) -> Result<SaleReceiptDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("sales.create"))?;
    if !role::user_has_permission(&connection, &context.user_id, "payments.record")
        .map_err(|error| error.to_string())?
    {
        return Err("Permission denied: payments.record".to_owned());
    }
    let open_shift = shift::get_open_shift(&connection, &context.business_id, &context.terminal_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Open a shift before completing a sale".to_owned())?;
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

#[cfg(test)]
mod tests {
    use super::POS_ITEM_QUERY;

    #[test]
    fn pos_item_query_matches_the_migrated_schema() {
        let database = localops_core::open_memory_database().unwrap();
        database.prepare(POS_ITEM_QUERY).unwrap();
    }
}

#[tauri::command]
pub fn get_sale_receipt(
    state: State<'_, DbState>,
    sale_id: String,
) -> Result<SaleReceiptDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    let owned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sales WHERE id = ?1 AND business_id = ?2)",
            (&sale_id, &context.business_id),
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !owned {
        return Err("Sale not found".to_owned());
    }
    sales::load_completed_sale(&connection, &sale_id)
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
