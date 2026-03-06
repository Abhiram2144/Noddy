use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    pub importance: f64,
    pub source: String,
}

pub fn create_memory(
    conn: &Connection,
    user_id: &str,
    content: String,
    _tags: Option<Vec<String>>,
) -> Result<String, String> {
    let now = current_timestamp();
    let uses_integer_id = memories_id_is_integer(conn)?;

    let id = if uses_integer_id {
        conn.execute(
            "INSERT INTO memories (user_id, content, created_at, updated_at, importance, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user_id, content, now, now, 0.5, "user_input"],
        )
        .map_err(|e| format!("Failed to create memory: {}", e))?;

        conn.last_insert_rowid().to_string()
    } else {
        let generated_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO memories (id, user_id, content, created_at, updated_at, importance, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![generated_id, user_id, content, now, now, 0.5, "user_input"],
        )
        .map_err(|e| format!("Failed to create memory: {}", e))?;
        generated_id
    };

    Ok(id)
}

pub fn get_memories(
    conn: &Connection,
    user_id: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<Memory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                    COALESCE(importance, 0.5),
                    COALESCE(source, 'user_input')
             FROM memories
             WHERE user_id = ?1 AND content IS NOT NULL
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let memories = stmt
        .query_map(params![user_id, limit, offset], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                importance: row.get(3)?,
                source: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map memories: {}", e))?;

    Ok(memories)
}

pub fn search_memories(
    conn: &Connection,
    user_id: &str,
    query: String,
    limit: i32,
) -> Result<Vec<Memory>, String> {
    let search_pattern = format!("%{}%", query);
    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                    COALESCE(importance, 0.5),
                    COALESCE(source, 'user_input')
             FROM memories
             WHERE user_id = ?1 AND content LIKE ?2 AND content IS NOT NULL
             ORDER BY importance DESC, created_at DESC
             LIMIT ?3",
        )
        .map_err(|e| format!("Failed to prepare search statement: {}", e))?;

    let memories = stmt
        .query_map(params![user_id, search_pattern, limit], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                importance: row.get(3)?,
                source: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query search results: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map search results: {}", e))?;

    Ok(memories)
}

pub fn delete_memory(conn: &Connection, user_id: &str, memory_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM memories WHERE id = ?1 AND user_id = ?2",
        params![memory_id, user_id],
    )
    .map_err(|e| format!("Failed to delete memory: {}", e))?;
    Ok(())
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}

fn memories_id_is_integer(conn: &Connection) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(memories)")
        .map_err(|e| format!("Failed to inspect memories schema: {}", e))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Failed to inspect memories schema: {}", e))?;

    while let Some(row) = rows
        .next()
        .map_err(|e| format!("Failed to inspect memories schema: {}", e))?
    {
        let name: String = row
            .get(1)
            .map_err(|e| format!("Failed to inspect memories schema: {}", e))?;
        if name == "id" {
            let data_type: String = row
                .get(2)
                .map_err(|e| format!("Failed to inspect memories schema: {}", e))?;
            return Ok(data_type.to_uppercase().contains("INT"));
        }
    }

    Ok(false)
}
