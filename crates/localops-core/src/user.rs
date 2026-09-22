use crate::{CoreError, Result};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

/// User entity representing a local system user with PIN authentication
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub business_id: String,
    pub username: String,
    pub display_name: String,
    pub pin_hash: String,
    pub role_id: Option<String>,
    pub active: bool,
    pub created_at: String,
}

/// Create a new user with PIN
pub fn create_user(
    connection: &Connection,
    business_id: &str,
    username: &str,
    display_name: &str,
    pin: &str,
) -> Result<String> {
    let username = username.trim();
    let display_name = display_name.trim();
    
    if username.is_empty() {
        return Err(CoreError::EmptyUsername);
    }
    
    if display_name.is_empty() {
        return Err(CoreError::EmptyDisplayName);
    }
    
    if pin.len() < 4 {
        return Err(CoreError::PinTooShort);
    }

    // Verify business exists
    let exists: bool = connection.query_row(
        "SELECT 1 FROM businesses WHERE id = ?1 AND active = 1",
        [business_id],
        |row| row.get::<_, i32>(0),
    ).optional()?.is_some();

    if !exists {
        return Err(CoreError::BusinessNotFound);
    }

    let id = Uuid::now_v7().to_string();
    let pin_hash = hash_pin(pin);
    
    connection.execute(
        "INSERT INTO users(id, business_id, username, display_name, pin_hash) VALUES(?1, ?2, ?3, ?4, ?5)",
        (&id, business_id, username, display_name, &pin_hash),
    )?;
    Ok(id)
}

/// Authenticate user with PIN
pub fn authenticate_user(connection: &Connection, username: &str, pin: &str) -> Result<Option<User>> {
    let pin_hash = hash_pin(pin);
    
    let mut stmt = connection.prepare(
        "SELECT id, business_id, username, display_name, pin_hash, role_id, active, created_at 
         FROM users WHERE username = ?1 AND pin_hash = ?2 AND active = 1"
    )?;
    
    let user = stmt.query_row([&username, pin_hash.as_str()], |row| {
        Ok(User {
            id: row.get(0)?,
            business_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            pin_hash: row.get(4)?,
            role_id: row.get(5)?,
            active: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).optional()?;
    
    Ok(user)
}

/// Get user by ID
pub fn get_user(connection: &Connection, id: &str) -> Result<Option<User>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, username, display_name, pin_hash, role_id, active, created_at 
         FROM users WHERE id = ?1"
    )?;
    
    let user = stmt.query_row([&id], |row| {
        Ok(User {
            id: row.get(0)?,
            business_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            pin_hash: row.get(4)?,
            role_id: row.get(5)?,
            active: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).optional()?;
    
    Ok(user)
}

/// Update user details
pub fn update_user(
    connection: &Connection,
    id: &str,
    display_name: Option<&str>,
    role_id: Option<&str>,
) -> Result<()> {
    if let Some(name) = display_name {
        if name.trim().is_empty() {
            return Err(CoreError::EmptyDisplayName);
        }
    }

    connection.execute(
        "UPDATE users 
         SET display_name = COALESCE(?1, display_name),
             role_id = COALESCE(?2, role_id)
         WHERE id = ?3",
        (display_name, role_id, id),
    )?;
    Ok(())
}

/// Change user PIN
pub fn change_user_pin(connection: &Connection, id: &str, new_pin: &str) -> Result<()> {
    if new_pin.len() < 4 {
        return Err(CoreError::PinTooShort);
    }

    let pin_hash = hash_pin(new_pin);
    connection.execute(
        "UPDATE users SET pin_hash = ?1 WHERE id = ?2",
        (&pin_hash, id),
    )?;
    Ok(())
}

/// Deactivate a user (soft delete)
pub fn deactivate_user(connection: &Connection, id: &str) -> Result<()> {
    connection.execute(
        "UPDATE users SET active = 0 WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

/// List all active users for a business
pub fn list_users(connection: &Connection, business_id: &str) -> Result<Vec<User>> {
    let mut stmt = connection.prepare(
        "SELECT id, business_id, username, display_name, pin_hash, role_id, active, created_at 
         FROM users 
         WHERE business_id = ?1 AND active = 1 
         ORDER BY display_name"
    )?;
    
    let users = stmt.query_map([business_id], |row| {
        Ok(User {
            id: row.get(0)?,
            business_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            pin_hash: row.get(4)?,
            role_id: row.get(5)?,
            active: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    
    users.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
}

/// Simple PIN hash (in production, use proper hashing like bcrypt/argon2)
fn hash_pin(pin: &str) -> String {
    // For now, simple SHA-256 hash - in production use proper password hashing
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_memory_database, business};

    #[test]
    fn creates_user_with_pin() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        let id = create_user(&db, &business_id, "john.doe", "John Doe", "1234").unwrap();
        
        let user = get_user(&db, &id).unwrap().unwrap();
        assert_eq!(user.username, "john.doe");
        assert_eq!(user.display_name, "John Doe");
        assert_eq!(user.business_id, business_id);
        assert!(user.active);
        assert!(!user.pin_hash.is_empty());
    }

    #[test]
    fn rejects_empty_username() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        assert!(matches!(
            create_user(&db, &business_id, "   ", "John Doe", "1234"),
            Err(CoreError::EmptyUsername)
        ));
    }

    #[test]
    fn rejects_empty_display_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        assert!(matches!(
            create_user(&db, &business_id, "john.doe", "   ", "1234"),
            Err(CoreError::EmptyDisplayName)
        ));
    }

    #[test]
    fn rejects_short_pin() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        assert!(matches!(
            create_user(&db, &business_id, "john.doe", "John Doe", "123"),
            Err(CoreError::PinTooShort)
        ));
    }

    #[test]
    fn authenticates_user_with_correct_pin() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        create_user(&db, &business_id, "john.doe", "John Doe", "1234").unwrap();
        
        let user = authenticate_user(&db, "john.doe", "1234").unwrap().unwrap();
        assert_eq!(user.username, "john.doe");
        assert_eq!(user.display_name, "John Doe");
        
        // Wrong PIN should fail
        let failed = authenticate_user(&db, "john.doe", "9999").unwrap();
        assert!(failed.is_none());
    }

    #[test]
    fn updates_user_details() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_user(&db, &business_id, "john.doe", "John Doe", "1234").unwrap();
        
        update_user(&db, &id, Some("John Updated"), None).unwrap();
        
        let user = get_user(&db, &id).unwrap().unwrap();
        assert_eq!(user.display_name, "John Updated");
        assert_eq!(user.username, "john.doe"); // Username unchanged
    }

    #[test]
    fn changes_user_pin() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_user(&db, &business_id, "john.doe", "John Doe", "1234").unwrap();
        
        // Old PIN should work
        assert!(authenticate_user(&db, "john.doe", "1234").unwrap().is_some());
        
        // Change PIN
        change_user_pin(&db, &id, "9876").unwrap();
        
        // New PIN should work
        assert!(authenticate_user(&db, "john.doe", "9876").unwrap().is_some());
        // Old PIN should fail
        assert!(authenticate_user(&db, "john.doe", "1234").unwrap().is_none());
    }

    #[test]
    fn deactivates_user() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        let id = create_user(&db, &business_id, "john.doe", "John Doe", "1234").unwrap();
        
        deactivate_user(&db, &id).unwrap();
        
        let user = get_user(&db, &id).unwrap().unwrap();
        assert!(!user.active);
        
        // Should not be able to authenticate inactive user
        let auth = authenticate_user(&db, "john.doe", "1234").unwrap();
        assert!(auth.is_none());
        
        // Should not appear in active list
        let users = list_users(&db, &business_id).unwrap();
        assert!(!users.iter().any(|u| u.id == id));
    }

    #[test]
    fn lists_active_users_ordered_by_name() {
        let db = open_memory_database().unwrap();
        let business_id = business::create_business(&db, "Test Business").unwrap();
        
        create_user(&db, &business_id, "charlie", "Charlie Brown", "1234").unwrap();
        create_user(&db, &business_id, "alice", "Alice Smith", "1234").unwrap();
        create_user(&db, &business_id, "bob", "Bob Jones", "1234").unwrap();
        
        let users = list_users(&db, &business_id).unwrap();
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].display_name, "Alice Smith");
        assert_eq!(users[1].display_name, "Bob Jones");
        assert_eq!(users[2].display_name, "Charlie Brown");
    }

    #[test]
    fn filters_users_by_business() {
        let db = open_memory_database().unwrap();
        let business_a = business::create_business(&db, "Business A").unwrap();
        let business_b = business::create_business(&db, "Business B").unwrap();
        
        create_user(&db, &business_a, "alice.a", "Alice A", "1234").unwrap();
        create_user(&db, &business_a, "bob.a", "Bob A", "1234").unwrap();
        create_user(&db, &business_b, "alice.b", "Alice B", "1234").unwrap();
        
        let users_a = list_users(&db, &business_a).unwrap();
        let users_b = list_users(&db, &business_b).unwrap();
        
        assert_eq!(users_a.len(), 2);
        assert_eq!(users_b.len(), 1);
        assert!(users_a.iter().all(|u| u.business_id == business_a));
        assert!(users_b.iter().all(|u| u.business_id == business_b));
    }
}
