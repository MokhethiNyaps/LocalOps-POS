use localops_core::report;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRange {
    start_at: String,
    end_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTotalsDto {
    gross_sales_minor: i64,
    refunds_minor: i64,
    net_sales_minor: i64,
    gross_profit_minor: i64,
    expenses_minor: i64,
    shift_variance_minor: i64,
    inventory_value_minor: i64,
    low_stock_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentSummaryDto {
    department_id: String,
    department_name: String,
    gross_sales_minor: i64,
    refunds_minor: i64,
    net_sales_minor: i64,
    gross_profit_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentSummaryDto {
    method_name: String,
    method_kind: String,
    received_minor: i64,
    refunded_minor: i64,
    net_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockSummaryDto {
    product_name: String,
    location_name: String,
    quantity_micros: i64,
    minimum_quantity_micros: i64,
    value_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    start_at: String,
    end_at: String,
    totals: ReportTotalsDto,
    departments: Vec<DepartmentSummaryDto>,
    payments: Vec<PaymentSummaryDto>,
    stock: Vec<StockSummaryDto>,
}

impl From<report::DashboardReport> for DashboardDto {
    fn from(value: report::DashboardReport) -> Self {
        Self {
            start_at: value.start_at,
            end_at: value.end_at,
            totals: ReportTotalsDto {
                gross_sales_minor: value.totals.gross_sales_minor,
                refunds_minor: value.totals.refunds_minor,
                net_sales_minor: value.totals.net_sales_minor,
                gross_profit_minor: value.totals.gross_profit_minor,
                expenses_minor: value.totals.expenses_minor,
                shift_variance_minor: value.totals.shift_variance_minor,
                inventory_value_minor: value.totals.inventory_value_minor,
                low_stock_count: value.totals.low_stock_count,
            },
            departments: value
                .departments
                .into_iter()
                .map(|item| DepartmentSummaryDto {
                    department_id: item.department_id,
                    department_name: item.department_name,
                    gross_sales_minor: item.gross_sales_minor,
                    refunds_minor: item.refunds_minor,
                    net_sales_minor: item.net_sales_minor,
                    gross_profit_minor: item.gross_profit_minor,
                })
                .collect(),
            payments: value
                .payments
                .into_iter()
                .map(|item| PaymentSummaryDto {
                    method_name: item.method_name,
                    method_kind: item.method_kind,
                    received_minor: item.received_minor,
                    refunded_minor: item.refunded_minor,
                    net_minor: item.net_minor,
                })
                .collect(),
            stock: value
                .stock
                .into_iter()
                .map(|item| StockSummaryDto {
                    product_name: item.product_name,
                    location_name: item.location_name,
                    quantity_micros: item.quantity_micros,
                    minimum_quantity_micros: item.minimum_quantity_micros,
                    value_minor: item.value_minor,
                })
                .collect(),
        }
    }
}

#[tauri::command]
pub fn get_dashboard_report(
    state: State<'_, DbState>,
    range: ReportRange,
) -> Result<DashboardDto, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("reports.view"))?;
    report::build_dashboard(
        &connection,
        &context.business_id,
        &range.start_at,
        &range.end_at,
    )
    .map(Into::into)
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_dashboard_csv(
    state: State<'_, DbState>,
    range: ReportRange,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("reports.view"))?;
    report::build_dashboard(
        &connection,
        &context.business_id,
        &range.start_at,
        &range.end_at,
    )
    .map(|value| report::to_csv(&value))
    .map_err(|error| error.to_string())
}
