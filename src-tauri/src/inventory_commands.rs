use localops_core::{inventory, inventory_operations, location, packaging, purchasing};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{session_guard::require_active_context, DbState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryLocationDto {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryPackagingDto {
    id: String,
    unit_id: String,
    name: String,
    factor_num: i64,
    factor_den: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryProductDto {
    id: String,
    name: String,
    base_unit_id: String,
    base_unit_code: String,
    packaging: Vec<InventoryPackagingDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryBalanceDto {
    product_id: String,
    product_name: String,
    location_id: String,
    location_name: String,
    quantity_micros: i64,
    version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierDto {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovementDto {
    id: String,
    product_name: String,
    location_name: String,
    quantity_micros: i64,
    movement_type: String,
    occurred_at: String,
    reference_type: String,
    reference_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySnapshot {
    locations: Vec<InventoryLocationDto>,
    products: Vec<InventoryProductDto>,
    balances: Vec<InventoryBalanceDto>,
    suppliers: Vec<SupplierDto>,
    recent_movements: Vec<MovementDto>,
    reconciliation_difference_count: usize,
}

#[tauri::command]
pub fn get_inventory_snapshot(state: State<'_, DbState>) -> Result<InventorySnapshot, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    let locations = location::list_locations(&connection, &context.business_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| InventoryLocationDto {
            id: value.id,
            name: value.name,
        })
        .collect();
    let balances = inventory::list_balances(&connection, &context.business_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| InventoryBalanceDto {
            product_id: value.product_id,
            product_name: value.product_name,
            location_id: value.location_id,
            location_name: value.location_name,
            quantity_micros: value.quantity_micros,
            version: value.version,
        })
        .collect();
    let suppliers = purchasing::list_suppliers(&connection, &context.business_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| SupplierDto {
            id: value.id,
            name: value.name,
        })
        .collect();
    let mut product_statement = connection
        .prepare(
            "SELECT s.id, s.name, p.base_unit_id, u.code
             FROM products p
             JOIN sellable_items s ON s.id = p.sellable_id
             JOIN units u ON u.id = p.base_unit_id
             WHERE s.business_id = ?1 AND s.active = 1 AND p.track_stock = 1
             ORDER BY s.name",
        )
        .map_err(|error| error.to_string())?;
    let product_rows = product_statement
        .query_map([&context.business_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut products = Vec::new();
    for row in product_rows {
        let (id, name, base_unit_id, base_unit_code) = row.map_err(|error| error.to_string())?;
        let product_packaging = packaging::list_product_packaging(&connection, &id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|value| value.can_purchase)
            .map(|value| InventoryPackagingDto {
                id: value.id,
                unit_id: value.unit_id,
                name: value.name,
                factor_num: value.factor_num,
                factor_den: value.factor_den,
            })
            .collect();
        products.push(InventoryProductDto {
            id,
            name,
            base_unit_id,
            base_unit_code,
            packaging: product_packaging,
        });
    }
    let mut movement_statement = connection
        .prepare(
            "SELECT m.id, s.name, l.name, m.quantity_micros, m.type,
                    m.occurred_at, m.reference_type, m.reference_id
             FROM inventory_movements m
             JOIN sellable_items s ON s.id = m.product_id
             JOIN locations l ON l.id = m.location_id
             WHERE m.business_id = ?1
             ORDER BY m.occurred_at DESC, m.created_at DESC LIMIT 50",
        )
        .map_err(|error| error.to_string())?;
    let recent_movements = movement_statement
        .query_map([&context.business_id], |row| {
            Ok(MovementDto {
                id: row.get(0)?,
                product_name: row.get(1)?,
                location_name: row.get(2)?,
                quantity_micros: row.get(3)?,
                movement_type: row.get(4)?,
                occurred_at: row.get(5)?,
                reference_type: row.get(6)?,
                reference_id: row.get(7)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let reconciliation_difference_count =
        inventory::reconcile_balances(&connection, &context.business_id)
            .map_err(|error| error.to_string())?
            .len();
    Ok(InventorySnapshot {
        locations,
        products,
        balances,
        suppliers,
        recent_movements,
        reconciliation_difference_count,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierInput {
    name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

#[tauri::command]
pub fn create_inventory_supplier(
    state: State<'_, DbState>,
    input: SupplierInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("inventory.manage"))?;
    purchasing::create_supplier(
        &connection,
        purchasing::NewSupplier {
            business_id: &context.business_id,
            name: &input.name,
            contact_name: input.contact_name.as_deref(),
            email: input.email.as_deref(),
            phone: input.phone.as_deref(),
            user_id: &context.user_id,
            terminal_id: Some(&context.terminal_id),
        },
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseItemCommandInput {
    product_id: String,
    description: String,
    quantity_micros: i64,
    unit_id: String,
    packaging_id: Option<String>,
    unit_cost_minor: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivePurchaseInput {
    purchase_number: String,
    supplier_id: Option<String>,
    location_id: String,
    invoice_reference: Option<String>,
    purchased_at: String,
    tax_minor: i64,
    discount_minor: i64,
    payment_status: String,
    notes: Option<String>,
    items: Vec<PurchaseItemCommandInput>,
}

#[tauri::command]
pub fn receive_inventory_purchase(
    state: State<'_, DbState>,
    input: ReceivePurchaseInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("inventory.manage"))?;
    let items = input
        .items
        .iter()
        .map(|item| purchasing::PurchaseItemInput {
            product_id: &item.product_id,
            description: &item.description,
            quantity_micros: item.quantity_micros,
            unit_id: &item.unit_id,
            packaging_id: item.packaging_id.as_deref(),
            unit_cost_minor: item.unit_cost_minor,
        })
        .collect::<Vec<_>>();
    purchasing::receive_purchase(
        &connection,
        purchasing::ReceivePurchase {
            business_id: &context.business_id,
            purchase_number: &input.purchase_number,
            supplier_id: input.supplier_id.as_deref(),
            location_id: &input.location_id,
            invoice_reference: input.invoice_reference.as_deref(),
            purchased_at: &input.purchased_at,
            tax_minor: input.tax_minor,
            discount_minor: input.discount_minor,
            payment_status: &input.payment_status,
            notes: input.notes.as_deref(),
            user_id: &context.user_id,
            terminal_id: Some(&context.terminal_id),
            items: &items,
        },
    )
    .map(|value| value.id)
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockItemCommandInput {
    product_id: String,
    quantity_micros: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferInput {
    from_location_id: String,
    to_location_id: String,
    occurred_at: String,
    notes: Option<String>,
    items: Vec<StockItemCommandInput>,
}

#[tauri::command]
pub fn complete_inventory_transfer(
    state: State<'_, DbState>,
    input: TransferInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("inventory.manage"))?;
    let items = input
        .items
        .iter()
        .map(|item| inventory_operations::StockItemInput {
            product_id: &item.product_id,
            quantity_micros: item.quantity_micros,
        })
        .collect::<Vec<_>>();
    inventory_operations::complete_transfer(
        &connection,
        inventory_operations::CompleteTransfer {
            business_id: &context.business_id,
            from_location_id: &input.from_location_id,
            to_location_id: &input.to_location_id,
            occurred_at: &input.occurred_at,
            user_id: &context.user_id,
            terminal_id: Some(&context.terminal_id),
            notes: input.notes.as_deref(),
            items: &items,
        },
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WastageInput {
    location_id: String,
    reason: String,
    occurred_at: String,
    notes: Option<String>,
    damage: bool,
    items: Vec<StockItemCommandInput>,
}

#[tauri::command]
pub fn record_inventory_wastage(
    state: State<'_, DbState>,
    input: WastageInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("inventory.manage"))?;
    let items = input
        .items
        .iter()
        .map(|item| inventory_operations::StockItemInput {
            product_id: &item.product_id,
            quantity_micros: item.quantity_micros,
        })
        .collect::<Vec<_>>();
    inventory_operations::record_wastage(
        &connection,
        inventory_operations::RecordWastage {
            business_id: &context.business_id,
            location_id: &input.location_id,
            reason: &input.reason,
            occurred_at: &input.occurred_at,
            user_id: &context.user_id,
            terminal_id: Some(&context.terminal_id),
            notes: input.notes.as_deref(),
            damage: input.damage,
            items: &items,
        },
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountedItemCommandInput {
    product_id: String,
    counted_micros: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockCountInput {
    location_id: String,
    counted_at: String,
    notes: Option<String>,
    items: Vec<CountedItemCommandInput>,
}

#[tauri::command]
pub fn complete_inventory_stock_count(
    state: State<'_, DbState>,
    input: StockCountInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("inventory.manage"))?;
    let items = input
        .items
        .iter()
        .map(|item| inventory_operations::CountedItemInput {
            product_id: &item.product_id,
            counted_micros: item.counted_micros,
        })
        .collect::<Vec<_>>();
    inventory_operations::complete_stock_count(
        &connection,
        inventory_operations::CompleteStockCount {
            business_id: &context.business_id,
            location_id: &input.location_id,
            counted_at: &input.counted_at,
            user_id: &context.user_id,
            terminal_id: Some(&context.terminal_id),
            notes: input.notes.as_deref(),
            items: &items,
        },
    )
    .map(|value| value.id)
    .map_err(|error| error.to_string())
}
