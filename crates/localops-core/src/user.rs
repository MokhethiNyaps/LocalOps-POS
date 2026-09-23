use crate::{CoreError, Result};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MAX_FAILED_ATTEMPTS: i64 = 5;
const LOCKOUT_MINUTES: i64 = 5;

/// Public user data. PIN hashes never leave the identity service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub business_id: String,
    pub username: String,
    pub display_name: String,
    pub active: bool,
    pub created_at: String,
}

pub fn create_user(
    connection: &Connection,
    business_id: &str,
    username: &str,
    display_name: &str,
    pin: &str,
) -> Result<String> {
    let username = username.trim();
    let display_name = display_name.trim();
    validate_user_input(username, display_name, pin)?;

    let exists = connection
        .query_row(
            "SELECT 1 FROM businesses WHERE id = ?1 AND active = 1",
            [business_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !exists {
        return Err(CoreError::BusinessNotFound);
    }

    let id = Uuid::now_v7().to_string();
    let pin_hash = hash_pin(pin)?;
    connection.execute(
        "INSERT INTO users(id, business_id, username, display_name, pin_hash)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        (&id, business_id, username, display_name, &pin_hash),
    )?;
    Ok(id)
}

/// Authenticate within one business. Five failed attempts lock the account for five minutes.
/// A legacy SHA-256 hash is upgraded after a successful login.
pub fn authenticate_user(
    connection: &Connection,
    business_id: &str,
    username: &str,
    pin: &str,
) -> Result<Option<User>> {
    if !valid_pin(pin) {
        return Ok(None);
    }

    let record = connection
        .query_row(
            "SELECT id, business_id, username, display_name, pin_hash, active, created_at,
                    CASE WHEN locked_until IS NOT NULL
                              AND locked_until > strftime('%Y-%m-%dT%H:%M:%fZ','now')
                         THEN 1 ELSE 0 END
             FROM users
             WHERE business_id = ?1 AND username = ?2 AND active = 1",
            (business_id, username.trim()),
            |row| {
                Ok((
                    User {
                        id: row.get(0)?,
                        business_id: row.get(1)?,
                        username: row.get(2)?,
                        display_name: row.get(3)?,
                        active: row.get(5)?,
                        created_at: row.get(6)?,
                    },
                    row.get::<_, String>(4)?,
                    row.get::<_, bool>(7)?,
                ))
            },
        )
        .optional()?;

    let Some((user, stored_hash, locked)) = record else {
        return Ok(None);
    };
    if locked {
        return Ok(None);
    }

    if verify_pin(&stored_hash, pin) {
        let upgraded_hash = if stored_hash.starts_with("$argon2id$") {
            None
        } else {
            Some(hash_pin(pin)?)
        };
        connection.execute(
            "UPDATE users
             SET failed_attempts = 0,
                 locked_until = NULL,
                 pin_hash = COALESCE(?1, pin_hash),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?2",
            (upgraded_hash.as_deref(), &user.id),
        )?;
        return Ok(Some(user));
    }

    connection.execute(
        "UPDATE users
         SET failed_attempts = failed_attempts + 1,
             locked_until = CASE
                 WHEN failed_attempts + 1 >= ?1
                 THEN strftime('%Y-%m-%dT%H:%M:%fZ','now', ?2)
                 ELSE locked_until
             END,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?3",
        (
            MAX_FAILED_ATTEMPTS,
            format!("+{LOCKOUT_MINUTES} minutes"),
            &user.id,
        ),
    )?;
    Ok(None)
}

pub fn get_user(connection: &Connection, id: &str) -> Result<Option<User>> {
    connection
        .query_row(
            "SELECT id, business_id, username, display_name, active, created_at
             FROM users WHERE id = ?1",
            [id],
            map_user,
        )
        .optional()
        .map_err(Into::into)
}

pub fn update_user(connection: &Connection, id: &str, display_name: &str) -> Result<()> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(CoreError::EmptyDisplayName);
    }
    let changed = connection.execute(
        "UPDATE users
         SET display_name = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?2",
        (display_name, id),
    )?;
    if changed == 0 {
        return Err(CoreError::UserNotFound);
    }
    Ok(())
}

pub fn change_user_pin(connection: &Connection, id: &str, new_pin: &str) -> Result<()> {
    if !valid_pin(new_pin) {
        return Err(CoreError::InvalidPin);
    }
    let pin_hash = hash_pin(new_pin)?;
    let changed = connection.execute(
        "UPDATE users
         SET pin_hash = ?1, failed_attempts = 0, locked_until = NULL,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?2",
        (&pin_hash, id),
    )?;
    if changed == 0 {
        return Err(CoreError::UserNotFound);
    }
    Ok(())
}

pub fn deactivate_user(connection: &Connection, id: &str) -> Result<()> {
    let changed = connection.execute(
        "UPDATE users
         SET active = 0, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1",
        [id],
    )?;
    if changed == 0 {
        return Err(CoreError::UserNotFound);
    }
    Ok(())
}

pub fn list_users(connection: &Connection, business_id: &str) -> Result<Vec<User>> {
    let mut statement = connection.prepare(
        "SELECT id, business_id, username, display_name, active, created_at
         FROM users WHERE business_id = ?1 AND active = 1 ORDER BY display_name",
    )?;
    statement
        .query_map([business_id], map_user)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        business_id: row.get(1)?,
        username: row.get(2)?,
        display_name: row.get(3)?,
        active: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn validate_user_input(username: &str, display_name: &str, pin: &str) -> Result<()> {
    if username.is_empty() {
        return Err(CoreError::EmptyUsername);
    }
    if display_name.is_empty() {
        return Err(CoreError::EmptyDisplayName);
    }
    if !valid_pin(pin) {
        return Err(CoreError::InvalidPin);
    }
    Ok(())
}

fn valid_pin(pin: &str) -> bool {
    (4..=12).contains(&pin.len()) && pin.bytes().all(|byte| byte.is_ascii_digit())
}

fn hash_pin(pin: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| CoreError::PinHashingFailed(error.to_string()))
}

fn verify_pin(stored_hash: &str, pin: &str) -> bool {
    if stored_hash.starts_with("$argon2id$") {
        return PasswordHash::new(stored_hash)
            .ok()
            .and_then(|hash| {
                Argon2::default()
                    .verify_password(pin.as_bytes(), &hash)
                    .ok()
            })
            .is_some();
    }
    legacy_sha256(pin) == stored_hash
}

fn legacy_sha256(pin: &str) -> String {
    format!("{:x}", Sha256::digest(pin.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{business, open_memory_database};

    fn user_fixture() -> (Connection, String, String) {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        let user_id = create_user(&database, &business_id, "john.doe", "John Doe", "1234").unwrap();
        (database, business_id, user_id)
    }

    #[test]
    fn creates_user_with_argon2id_pin_hash() {
        let (database, _, user_id) = user_fixture();
        let hash: String = database
            .query_row(
                "SELECT pin_hash FROM users WHERE id = ?1",
                [&user_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains("1234"));
    }

    #[test]
    fn validates_pin_shape() {
        let database = open_memory_database().unwrap();
        let business_id = business::create_business(&database, "Test Business").unwrap();
        for invalid in ["123", "1234567890123", "12ab"] {
            assert!(matches!(
                create_user(&database, &business_id, "john", "John", invalid),
                Err(CoreError::InvalidPin)
            ));
        }
    }

    #[test]
    fn authentication_is_scoped_to_business() {
        let database = open_memory_database().unwrap();
        let business_a = business::create_business(&database, "Business A").unwrap();
        let business_b = business::create_business(&database, "Business B").unwrap();
        create_user(&database, &business_a, "cashier", "Cashier A", "1111").unwrap();
        create_user(&database, &business_b, "cashier", "Cashier B", "2222").unwrap();

        let user = authenticate_user(&database, &business_b, "cashier", "2222")
            .unwrap()
            .unwrap();
        assert_eq!(user.display_name, "Cashier B");
        assert!(
            authenticate_user(&database, &business_a, "cashier", "2222")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn locks_account_after_five_failed_attempts() {
        let (database, business_id, user_id) = user_fixture();
        for _ in 0..MAX_FAILED_ATTEMPTS {
            assert!(
                authenticate_user(&database, &business_id, "john.doe", "9999")
                    .unwrap()
                    .is_none()
            );
        }
        let locked_until: Option<String> = database
            .query_row(
                "SELECT locked_until FROM users WHERE id = ?1",
                [&user_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(locked_until.is_some());
        assert!(
            authenticate_user(&database, &business_id, "john.doe", "1234")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn upgrades_legacy_sha256_hash_after_successful_login() {
        let (database, business_id, user_id) = user_fixture();
        database
            .execute(
                "UPDATE users SET pin_hash = ?1 WHERE id = ?2",
                (legacy_sha256("1234"), &user_id),
            )
            .unwrap();

        assert!(
            authenticate_user(&database, &business_id, "john.doe", "1234")
                .unwrap()
                .is_some()
        );
        let hash: String = database
            .query_row(
                "SELECT pin_hash FROM users WHERE id = ?1",
                [&user_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn updates_pin_and_user_details() {
        let (database, business_id, user_id) = user_fixture();
        update_user(&database, &user_id, "John Updated").unwrap();
        change_user_pin(&database, &user_id, "9876").unwrap();
        let user = get_user(&database, &user_id).unwrap().unwrap();
        assert_eq!(user.display_name, "John Updated");
        assert!(
            authenticate_user(&database, &business_id, "john.doe", "9876")
                .unwrap()
                .is_some()
        );
        assert!(
            authenticate_user(&database, &business_id, "john.doe", "1234")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn deactivates_and_lists_users() {
        let (database, business_id, user_id) = user_fixture();
        create_user(&database, &business_id, "alice", "Alice", "4567").unwrap();
        deactivate_user(&database, &user_id).unwrap();
        let users = list_users(&database, &business_id).unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].display_name, "Alice");
        assert!(
            authenticate_user(&database, &business_id, "john.doe", "1234")
                .unwrap()
                .is_none()
        );
    }
}
