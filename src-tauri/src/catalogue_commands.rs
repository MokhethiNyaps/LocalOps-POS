use localops_core::{audit, availability, catalogue, category, packaging, recipe, unit};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    session_guard::{require_active_context, ActiveContext},
    DbState,
};

fn record_catalogue_audit(
    connection: &Connection,
    context: &ActiveContext,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    new_json: &str,
) -> Result<(), String> {
    audit::record_event(
        connection,
        audit::NewAuditEvent {
            business_id: &context.business_id,
            user_id: Some(&context.user_id),
            terminal_id: Some(&context.terminal_id),
            action,
            entity_type,
            entity_id,
            old_json: None,
            new_json: Some(new_json),
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    id: String,
    name: String,
    parent_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitDto {
    id: String,
    code: String,
    name: String,
    dimension: String,
    scale_num: i64,
    scale_den: i64,
    decimal_places: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagingDto {
    id: String,
    unit_id: String,
    name: String,
    factor_num: i64,
    factor_den: i64,
    can_purchase: bool,
    can_sell: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueItemDto {
    id: String,
    kind: String,
    name: String,
    category_id: Option<String>,
    price_minor: i64,
    taxable: bool,
    barcode: Option<String>,
    sku: Option<String>,
    product_code: Option<String>,
    base_unit_id: Option<String>,
    cost_minor: Option<i64>,
    track_stock: Option<bool>,
    duration_minutes: Option<i32>,
    has_recipe: bool,
    packaging: Vec<PackagingDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentDto {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueSnapshot {
    business_id: String,
    terminal_id: String,
    terminal_department_id: Option<String>,
    categories: Vec<CategoryDto>,
    units: Vec<UnitDto>,
    items: Vec<CatalogueItemDto>,
    departments: Vec<DepartmentDto>,
}

#[tauri::command]
pub fn get_catalogue_snapshot(state: State<'_, DbState>) -> Result<CatalogueSnapshot, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, None)?;
    let categories = category::get_categories_by_business(&connection, &context.business_id, true)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| CategoryDto {
            id: value.id,
            name: value.name,
            parent_id: value.parent_id,
        })
        .collect();
    let units = unit::get_units_by_business(&connection, &context.business_id, true)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|value| UnitDto {
            id: value.id,
            code: value.code,
            name: value.name,
            dimension: value.dimension,
            scale_num: value.scale_num,
            scale_den: value.scale_den,
            decimal_places: value.decimal_places,
        })
        .collect();
    let departments =
        localops_core::department::list_departments(&connection, &context.business_id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|value| DepartmentDto {
                id: value.id,
                name: value.name,
            })
            .collect();
    let sellables = localops_core::sellable::get_sellable_items_by_business(
        &connection,
        &context.business_id,
        true,
    )
    .map_err(|error| error.to_string())?;
    let mut items = Vec::with_capacity(sellables.len());
    for sellable in sellables {
        let product = localops_core::product::get_product(&connection, &sellable.id)
            .map_err(|error| error.to_string())?;
        let service = localops_core::service::get_service(&connection, &sellable.id)
            .map_err(|error| error.to_string())?;
        let item_packaging = packaging::list_product_packaging(&connection, &sellable.id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|value| PackagingDto {
                id: value.id,
                unit_id: value.unit_id,
                name: value.name,
                factor_num: value.factor_num,
                factor_den: value.factor_den,
                can_purchase: value.can_purchase,
                can_sell: value.can_sell,
            })
            .collect();
        let has_recipe = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM recipes WHERE owner_sellable_id = ?1 AND active = 1)",
                [&sellable.id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        items.push(CatalogueItemDto {
            id: sellable.id,
            kind: sellable.kind,
            name: sellable.name,
            category_id: sellable.category_id,
            price_minor: sellable.price_minor,
            taxable: sellable.taxable,
            barcode: sellable.barcode,
            sku: sellable.sku,
            product_code: sellable.product_code,
            base_unit_id: product.as_ref().map(|value| value.base_unit_id.clone()),
            cost_minor: product.as_ref().map(|value| value.cost_minor),
            track_stock: product.as_ref().map(|value| value.track_stock),
            duration_minutes: service.and_then(|value| value.duration_minutes),
            has_recipe,
            packaging: item_packaging,
        });
    }
    Ok(CatalogueSnapshot {
        business_id: context.business_id,
        terminal_id: context.terminal_id,
        terminal_department_id: context.department_id,
        categories,
        units,
        items,
        departments,
    })
}

#[tauri::command]
pub fn create_catalogue_category(
    state: State<'_, DbState>,
    name: String,
    parent_id: Option<String>,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let id = category::create_category(
        &transaction,
        &context.business_id,
        &name,
        parent_id.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({ "name": name, "parentId": parent_id }).to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "CATEGORY_CREATED",
        "category",
        &id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitInput {
    code: String,
    name: String,
    dimension: String,
    scale_num: i64,
    scale_den: i64,
    decimal_places: i32,
}

#[tauri::command]
pub fn create_catalogue_unit(
    state: State<'_, DbState>,
    input: UnitInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let id = unit::create_unit(
        &transaction,
        &context.business_id,
        &input.code,
        &input.name,
        &input.dimension,
        input.scale_num,
        input.scale_den,
        input.decimal_places,
    )
    .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "code": input.code,
        "name": input.name,
        "dimension": input.dimension,
        "scaleNum": input.scale_num,
        "scaleDen": input.scale_den
    })
    .to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "UNIT_CREATED",
        "unit",
        &id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    name: String,
    price_minor: i64,
    category_id: Option<String>,
    taxable: bool,
    base_unit_id: String,
    cost_minor: i64,
    track_stock: bool,
    minimum_quantity_micros: i64,
    barcode: Option<String>,
    sku: Option<String>,
    product_code: Option<String>,
}

#[tauri::command]
pub fn create_catalogue_product(
    state: State<'_, DbState>,
    input: ProductInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let id = catalogue::create_product(
        &transaction,
        catalogue::NewProduct {
            business_id: &context.business_id,
            name: &input.name,
            price_minor: input.price_minor,
            category_id: input.category_id.as_deref(),
            taxable: input.taxable,
            base_unit_id: &input.base_unit_id,
            cost_minor: input.cost_minor,
            track_stock: input.track_stock,
            minimum_quantity_micros: input.minimum_quantity_micros,
        },
    )
    .map_err(|error| error.to_string())?;
    localops_core::sellable::set_sellable_identifiers(
        &transaction,
        &id,
        input.barcode.as_deref(),
        input.sku.as_deref(),
        input.product_code.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "name": input.name,
        "priceMinor": input.price_minor,
        "baseUnitId": input.base_unit_id,
        "trackStock": input.track_stock,
        "barcode": input.barcode,
        "sku": input.sku,
        "productCode": input.product_code
    })
    .to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "PRODUCT_CREATED",
        "sellable_item",
        &id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInput {
    name: String,
    price_minor: i64,
    category_id: Option<String>,
    taxable: bool,
    duration_minutes: Option<i32>,
}

#[tauri::command]
pub fn create_catalogue_service(
    state: State<'_, DbState>,
    input: ServiceInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let id = catalogue::create_service(
        &transaction,
        catalogue::NewService {
            business_id: &context.business_id,
            name: &input.name,
            price_minor: input.price_minor,
            category_id: input.category_id.as_deref(),
            taxable: input.taxable,
            duration_minutes: input.duration_minutes,
        },
    )
    .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "name": input.name,
        "priceMinor": input.price_minor,
        "durationMinutes": input.duration_minutes
    })
    .to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "SERVICE_CREATED",
        "sellable_item",
        &id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagingInput {
    product_id: String,
    unit_id: String,
    name: String,
    factor_num: i64,
    factor_den: i64,
    can_purchase: bool,
    can_sell: bool,
}

#[tauri::command]
pub fn create_catalogue_packaging(
    state: State<'_, DbState>,
    input: PackagingInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let id = packaging::create_packaging(
        &transaction,
        &context.business_id,
        &input.product_id,
        &input.unit_id,
        &input.name,
        input.factor_num,
        input.factor_den,
        input.can_purchase,
        input.can_sell,
    )
    .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "productId": input.product_id,
        "name": input.name,
        "factorNum": input.factor_num,
        "factorDen": input.factor_den
    })
    .to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "PRODUCT_PACKAGING_CREATED",
        "product_packaging",
        &id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeItemInput {
    ingredient_product_id: String,
    quantity_micros: i64,
    unit_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeInput {
    owner_sellable_id: String,
    yield_quantity_micros: i64,
    items: Vec<RecipeItemInput>,
}

#[tauri::command]
pub fn replace_catalogue_recipe(
    state: State<'_, DbState>,
    input: RecipeInput,
) -> Result<String, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let items = input
        .items
        .iter()
        .map(|item| recipe::NewRecipeItem {
            ingredient_product_id: &item.ingredient_product_id,
            quantity_micros: item.quantity_micros,
            unit_id: &item.unit_id,
        })
        .collect::<Vec<_>>();
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let id = recipe::replace_recipe(
        &transaction,
        &context.business_id,
        &input.owner_sellable_id,
        input.yield_quantity_micros,
        &items,
    )
    .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "ownerSellableId": input.owner_sellable_id,
        "yieldQuantityMicros": input.yield_quantity_micros,
        "itemCount": input.items.len()
    })
    .to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "RECIPE_REPLACED",
        "recipe",
        &id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityInput {
    department_id: String,
    sellable_id: String,
    price_override_minor: Option<i64>,
    active: bool,
}

#[tauri::command]
pub fn set_catalogue_availability(
    state: State<'_, DbState>,
    input: AvailabilityInput,
) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let context = require_active_context(&connection, &state, Some("products.manage"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    availability::set_department_availability(
        &transaction,
        &input.department_id,
        &input.sellable_id,
        input.price_override_minor,
        input.active,
    )
    .map_err(|error| error.to_string())?;
    let entity_id = format!("{}:{}", input.department_id, input.sellable_id);
    let payload = serde_json::json!({
        "departmentId": input.department_id,
        "sellableId": input.sellable_id,
        "priceOverrideMinor": input.price_override_minor,
        "active": input.active
    })
    .to_string();
    record_catalogue_audit(
        &transaction,
        &context,
        "DEPARTMENT_SELLABLE_SET",
        "department_sellable",
        &entity_id,
        &payload,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(())
}
