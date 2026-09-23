use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub id: String,
    pub business_id: String,
    pub name: String,
    pub system: bool,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub code: String,
    pub description: String,
}

pub fn create_role(
    connection: &Connection,
    business_id: &str,
    name: &str,
    system: bool,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::EmptyRoleName);
    }
    let business_exists = connection
        .query_row(
            "SELECT 1 FROM businesses WHERE id = ?1 AND active = 1",
            [business_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !business_exists {
        return Err(CoreError::BusinessNotFound);
    }

    let id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO roles(id, business_id, name, system) VALUES(?1, ?2, ?3, ?4)",
        (&id, business_id, name, system),
    )?;
    Ok(id)
}

pub fn list_roles(connection: &Connection, business_id: &str) -> Result<Vec<Role>> {
    let mut statement = connection.prepare(
        "SELECT id, business_id, name, system, active
         FROM roles WHERE business_id = ?1 AND active = 1 ORDER BY name",
    )?;
    statement
        .query_map([business_id], |row| {
            Ok(Role {
                id: row.get(0)?,
                business_id: row.get(1)?,
                name: row.get(2)?,
                system: row.get(3)?,
                active: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn list_permissions(connection: &Connection) -> Result<Vec<Permission>> {
    let mut statement =
        connection.prepare("SELECT code, description FROM permissions ORDER BY code")?;
    statement
        .query_map([], |row| {
            Ok(Permission {
                code: row.get(0)?,
                description: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn grant_permission(
    connection: &Connection,
    role_id: &str,
    permission_code: &str,
    granted_by: Option<&str>,
) -> Result<()> {
    if !permission_exists(connection, permission_code)? {
        return Err(CoreError::PermissionNotFound);
    }
    if !role_exists(connection, role_id)? {
        return Err(CoreError::RoleNotFound);
    }
    if let Some(actor_id) = granted_by {
        ensure_role_and_user_share_business(connection, role_id, actor_id)?;
    }
    connection.execute(
        "INSERT INTO role_permissions(role_id, permission_code, granted_by)
         VALUES(?1, ?2, ?3)
         ON CONFLICT(role_id, permission_code) DO NOTHING",
        (role_id, permission_code, granted_by),
    )?;
    Ok(())
}

pub fn revoke_permission(
    connection: &Connection,
    role_id: &str,
    permission_code: &str,
) -> Result<()> {
    connection.execute(
        "DELETE FROM role_permissions WHERE role_id = ?1 AND permission_code = ?2",
        (role_id, permission_code),
    )?;
    Ok(())
}

pub fn assign_role(
    connection: &Connection,
    user_id: &str,
    role_id: &str,
    assigned_by: Option<&str>,
) -> Result<()> {
    ensure_role_and_user_share_business(connection, role_id, user_id)?;
    if let Some(actor_id) = assigned_by {
        ensure_role_and_user_share_business(connection, role_id, actor_id)?;
    }
    connection.execute(
        "INSERT INTO user_roles(user_id, role_id, assigned_by)
         VALUES(?1, ?2, ?3)
         ON CONFLICT(user_id, role_id) DO NOTHING",
        (user_id, role_id, assigned_by),
    )?;
    Ok(())
}

pub fn unassign_role(connection: &Connection, user_id: &str, role_id: &str) -> Result<()> {
    connection.execute(
        "DELETE FROM user_roles WHERE user_id = ?1 AND role_id = ?2",
        (user_id, role_id),
    )?;
    Ok(())
}

pub fn user_has_permission(
    connection: &Connection,
    user_id: &str,
    permission_code: &str,
) -> Result<bool> {
    let granted = connection
        .query_row(
            "SELECT 1
             FROM user_roles ur
             JOIN users u ON u.id = ur.user_id AND u.active = 1
             JOIN roles r ON r.id = ur.role_id AND r.active = 1
             JOIN role_permissions rp ON rp.role_id = r.id
             WHERE ur.user_id = ?1 AND rp.permission_code = ?2
             LIMIT 1",
            (user_id, permission_code),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(granted)
}

pub fn list_user_permissions(connection: &Connection, user_id: &str) -> Result<Vec<Permission>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT p.code, p.description
         FROM user_roles ur
         JOIN roles r ON r.id = ur.role_id AND r.active = 1
         JOIN role_permissions rp ON rp.role_id = r.id
         JOIN permissions p ON p.code = rp.permission_code
         WHERE ur.user_id = ?1
         ORDER BY p.code",
    )?;
    statement
        .query_map([user_id], |row| {
            Ok(Permission {
                code: row.get(0)?,
                description: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn deactivate_role(connection: &Connection, role_id: &str) -> Result<()> {
    let changed = connection.execute(
        "UPDATE roles
         SET active = 0, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1 AND system = 0",
        [role_id],
    )?;
    if changed == 0 {
        return Err(CoreError::RoleNotFound);
    }
    Ok(())
}

fn role_exists(connection: &Connection, role_id: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM roles WHERE id = ?1 AND active = 1",
            [role_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn permission_exists(connection: &Connection, permission_code: &str) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM permissions WHERE code = ?1",
            [permission_code],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn ensure_role_and_user_share_business(
    connection: &Connection,
    role_id: &str,
    user_id: &str,
) -> Result<()> {
    let matches = connection
        .query_row(
            "SELECT 1
             FROM roles r JOIN users u ON u.business_id = r.business_id
             WHERE r.id = ?1 AND u.id = ?2 AND r.active = 1 AND u.active = 1",
            (role_id, user_id),
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !matches {
        return Err(CoreError::CrossBusinessReference);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database, user};

    #[test]
    fn assigns_many_roles_and_combines_permissions() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let user_id = user::create_user(&database, &business_id, "pat", "Pat", "1234").unwrap();
        let sales = create_role(&database, &business_id, "Sales", false).unwrap();
        let reports = create_role(&database, &business_id, "Reports", false).unwrap();
        grant_permission(&database, &sales, "sales.create", None).unwrap();
        grant_permission(&database, &reports, "reports.view", None).unwrap();
        assign_role(&database, &user_id, &sales, None).unwrap();
        assign_role(&database, &user_id, &reports, None).unwrap();

        assert!(user_has_permission(&database, &user_id, "sales.create").unwrap());
        assert!(user_has_permission(&database, &user_id, "reports.view").unwrap());
        assert_eq!(list_user_permissions(&database, &user_id).unwrap().len(), 2);
    }

    #[test]
    fn blocks_cross_business_role_assignment() {
        let database = open_memory_database().unwrap();
        let business_a = business::create_business(&database, "Business A").unwrap();
        let business_b = business::create_business(&database, "Business B").unwrap();
        let user_id = user::create_user(&database, &business_a, "pat", "Pat", "1234").unwrap();
        let role_id = create_role(&database, &business_b, "Manager", false).unwrap();
        assert!(matches!(
            assign_role(&database, &user_id, &role_id, None),
            Err(CoreError::CrossBusinessReference)
        ));
    }

    #[test]
    fn revoking_permission_removes_authority() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let user_id = user::create_user(&database, &business_id, "pat", "Pat", "1234").unwrap();
        let role_id = create_role(&database, &business_id, "Sales", false).unwrap();
        grant_permission(&database, &role_id, "sales.create", None).unwrap();
        assign_role(&database, &user_id, &role_id, None).unwrap();
        revoke_permission(&database, &role_id, "sales.create").unwrap();
        assert!(!user_has_permission(&database, &user_id, "sales.create").unwrap());
    }

    #[test]
    fn lists_roles_and_permission_catalogue() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        create_role(&database, &business_id, "Manager", false).unwrap();
        assert_eq!(list_roles(&database, &business_id).unwrap().len(), 1);
        assert!(
            list_permissions(&database)
                .unwrap()
                .iter()
                .any(|permission| permission.code == "products.manage")
        );
    }
}
