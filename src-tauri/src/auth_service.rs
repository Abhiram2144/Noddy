use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const ACCESS_TOKEN_TTL_SECONDS: i64 = 15 * 60;
const REFRESH_TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthResult {
    pub user: AuthUser,
    pub tokens: AuthTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
    jti: String,
}

pub fn signup(conn: &Connection, email: &str, password: &str, jwt_secret: &str) -> Result<AuthResult, String> {
    validate_email(email)?;
    validate_password(password)?;

    let normalized_email = email.trim().to_lowercase();
    let now = now_timestamp();

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM users WHERE email = ?1",
            params![normalized_email],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to check existing user: {}", e))?;

    if existing.is_some() {
        return Err("A user with this email already exists".to_string());
    }

    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(password)?;

    conn.execute(
        "INSERT INTO users (id, email, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![user_id, normalized_email, password_hash, now],
    )
    .map_err(|e| format!("Failed to create user: {}", e))?;

    // Migrate legacy local data created before multi-user support.
    claim_orphaned_local_data(conn, &user_id)?;

    let tokens = issue_tokens(conn, &user_id, jwt_secret)?;

    Ok(AuthResult {
        user: AuthUser {
            id: user_id,
            email: normalized_email,
            created_at: now,
        },
        tokens,
    })
}

pub fn login(conn: &Connection, email: &str, password: &str, jwt_secret: &str) -> Result<AuthResult, String> {
    let normalized_email = email.trim().to_lowercase();

    let user = conn
        .query_row(
            "SELECT id, email, password_hash, created_at FROM users WHERE email = ?1",
            params![normalized_email],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| format!("Failed to load user: {}", e))?
        .ok_or_else(|| "Invalid email or password".to_string())?;

    verify_password(&user.2, password)?;

    // Ensure previously unowned local rows are attached to this account.
    claim_orphaned_local_data(conn, &user.0)?;

    let tokens = issue_tokens(conn, &user.0, jwt_secret)?;

    Ok(AuthResult {
        user: AuthUser {
            id: user.0,
            email: user.1,
            created_at: user.3,
        },
        tokens,
    })
}

pub fn refresh(conn: &Connection, refresh_token: &str, jwt_secret: &str) -> Result<AuthTokens, String> {
    let user_id = verify_jwt(refresh_token, jwt_secret)?;
    let now = now_timestamp();

    let session_exists: Option<i64> = conn
        .query_row(
            "SELECT expires_at FROM sessions WHERE refresh_token = ?1 AND user_id = ?2",
            params![refresh_token, user_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to validate refresh session: {}", e))?;

    let expires_at = session_exists.ok_or_else(|| "Refresh session not found".to_string())?;

    if expires_at <= now {
        conn.execute(
            "DELETE FROM sessions WHERE refresh_token = ?1",
            params![refresh_token],
        )
        .map_err(|e| format!("Failed to delete expired session: {}", e))?;
        return Err("Refresh token expired".to_string());
    }

    conn.execute(
        "DELETE FROM sessions WHERE refresh_token = ?1",
        params![refresh_token],
    )
    .map_err(|e| format!("Failed to rotate refresh token: {}", e))?;

    issue_tokens(conn, &user_id, jwt_secret)
}

pub fn logout(conn: &Connection, refresh_token: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM sessions WHERE refresh_token = ?1",
        params![refresh_token],
    )
    .map_err(|e| format!("Failed to logout session: {}", e))?;
    Ok(())
}

pub fn verify_access_token(token: &str, jwt_secret: &str) -> Result<String, String> {
    verify_jwt(token, jwt_secret)
}

pub fn get_user_by_id(conn: &Connection, user_id: &str) -> Result<AuthUser, String> {
    conn.query_row(
        "SELECT id, email, created_at FROM users WHERE id = ?1",
        params![user_id],
        |row| {
            Ok(AuthUser {
                id: row.get(0)?,
                email: row.get(1)?,
                created_at: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("Failed to fetch user: {}", e))?
    .ok_or_else(|| "User not found".to_string())
}

pub fn claim_orphaned_data_for_user(conn: &Connection, user_id: &str) -> Result<(), String> {
    claim_orphaned_local_data(conn, user_id)
}

fn issue_tokens(conn: &Connection, user_id: &str, jwt_secret: &str) -> Result<AuthTokens, String> {
    let now = now_timestamp();
    let access_token = sign_jwt(user_id, now + ACCESS_TOKEN_TTL_SECONDS, jwt_secret)?;
    let refresh_token = sign_jwt(user_id, now + REFRESH_TOKEN_TTL_SECONDS, jwt_secret)?;

    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (id, user_id, refresh_token, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, user_id, refresh_token, now, now + REFRESH_TOKEN_TTL_SECONDS],
    )
    .map_err(|e| format!("Failed to persist refresh session: {}", e))?;

    Ok(AuthTokens {
        access_token,
        refresh_token,
        expires_in: ACCESS_TOKEN_TTL_SECONDS,
        token_type: "Bearer".to_string(),
        user_id: user_id.to_string(),
    })
}

fn sign_jwt(user_id: &str, exp: i64, jwt_secret: &str) -> Result<String, String> {
    let now = now_timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        exp: exp as usize,
        iat: now as usize,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to sign JWT: {}", e))
}

fn verify_jwt(token: &str, jwt_secret: &str) -> Result<String, String> {
    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| "Invalid or expired token".to_string())?;

    Ok(decoded.claims.sub)
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Failed to hash password: {}", e))
}

fn verify_password(stored_hash: &str, password: &str) -> Result<(), String> {
    let parsed_hash = PasswordHash::new(stored_hash)
        .map_err(|e| format!("Stored password hash is invalid: {}", e))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| "Invalid email or password".to_string())
}

fn validate_email(email: &str) -> Result<(), String> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') || email.len() > 254 {
        return Err("Please provide a valid email address".to_string());
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    Ok(())
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn claim_orphaned_local_data(conn: &Connection, user_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE memories SET user_id = ?1 WHERE user_id IS NULL",
        params![user_id],
    )
    .map_err(|e| format!("Failed to migrate legacy memories: {}", e))?;

    conn.execute(
        "UPDATE reminders SET user_id = ?1 WHERE user_id IS NULL",
        params![user_id],
    )
    .map_err(|e| format!("Failed to migrate legacy reminders: {}", e))?;

    conn.execute(
        "UPDATE command_history SET user_id = ?1 WHERE user_id IS NULL",
        params![user_id],
    )
    .map_err(|e| format!("Failed to migrate legacy history: {}", e))?;

    conn.execute(
        "UPDATE memory_edges SET user_id = ?1 WHERE user_id IS NULL",
        params![user_id],
    )
    .map_err(|e| format!("Failed to migrate legacy edges: {}", e))?;

    Ok(())
}
