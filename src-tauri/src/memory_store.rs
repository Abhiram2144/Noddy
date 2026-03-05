use rusqlite::{Connection, params, OptionalExtension};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Service module for memory CRUD operations
/// Provides abstraction over database access for memory-related functionality

#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub importance: f64,
    pub source: String,
}

/// Create a new memory with optional tags
pub fn create_memory(
    conn: &Connection,
    content: String,
    _tags: Option<Vec<String>>,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();
    
    conn.execute(
        "INSERT INTO memories (id, content, created_at, updated_at, importance, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, content, now, now, 0.5, "user_input"],
    )
    .map_err(|e| format!("Failed to create memory: {}", e))?;
    
    Ok(id)
}

/// Retrieve memories with pagination
pub fn get_memories(
    conn: &Connection,
    limit: i32,
    offset: i32,
) -> Result<Vec<Memory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, updated_at, importance, source
             FROM memories
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let memories = stmt
        .query_map(params![limit, offset], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                importance: row.get(4)?,
                source: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map memories: {}", e))?;
    
    Ok(memories)
}

/// Search memories by keyword
pub fn search_memories(
    conn: &Connection,
    query: String,
    limit: i32,
) -> Result<Vec<Memory>, String> {
    let search_pattern = format!("%{}%", query);
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, updated_at, importance, source
             FROM memories
             WHERE content LIKE ?1
             ORDER BY importance DESC, created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare search statement: {}", e))?;
    
    let memories = stmt
        .query_map(params![search_pattern, limit], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                importance: row.get(4)?,
                source: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query search results: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map search results: {}", e))?;
    
    Ok(memories)
}

/// Update memory content and timestamp
pub fn update_memory(
    conn: &Connection,
    memory_id: &str,
    content: String,
) -> Result<(), String> {
    let now = current_timestamp();
    
    conn.execute(
        "UPDATE memories SET content = ?1, updated_at = ?2 WHERE id = ?3",
        params![content, now, memory_id],
    )
    .map_err(|e| format!("Failed to update memory: {}", e))?;
    
    Ok(())
}

/// Update memory importance score
pub fn update_memory_importance(
    conn: &Connection,
    memory_id: &str,
    importance: f64,
) -> Result<(), String> {
    let clamped_importance = importance.max(0.0).min(1.0);
    
    conn.execute(
        "UPDATE memories SET importance = ?1, updated_at = ?2 WHERE id = ?3",
        params![clamped_importance, current_timestamp(), memory_id],
    )
    .map_err(|e| format!("Failed to update memory importance: {}", e))?;
    
    Ok(())
}

/// Delete a memory and all associated data (tags, edges, reminders)
pub fn delete_memory(
    conn: &Connection,
    memory_id: &str,
) -> Result<(), String> {
    // Foreign key cascades will handle cleanup of:
    // - memory_tag_links
    // - memory_edges (source and target)
    // - reminders (linked to this memory)
    
    conn.execute(
        "DELETE FROM memories WHERE id = ?1",
        params![memory_id],
    )
    .map_err(|e| format!("Failed to delete memory: {}", e))?;
    
    Ok(())
}

/// Get a single memory by ID
pub fn get_memory_by_id(
    conn: &Connection,
    memory_id: &str,
) -> Result<Option<Memory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, updated_at, importance, source
             FROM memories
             WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let memory = stmt
        .query_row(params![memory_id], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                importance: row.get(4)?,
                source: row.get(5)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query memory: {}", e))?;
    
    Ok(memory)
}

/// Get all memories created today
pub fn get_memories_created_today(
    conn: &Connection,
) -> Result<Vec<Memory>, String> {
    let now = current_timestamp();
    let start_of_today = (now / 86400) * 86400; // Midnight UTC
    
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, updated_at, importance, source
             FROM memories
             WHERE created_at >= ?1
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let memories = stmt
        .query_map(params![start_of_today], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                importance: row.get(4)?,
                source: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map memories: {}", e))?;
    
    Ok(memories)
}

/// Get memory count
pub fn get_memory_count(conn: &Connection) -> Result<i64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count memories: {}", e))?;
    
    Ok(count)
}

/// Get top memories by importance
pub fn get_top_memories_by_importance(
    conn: &Connection,
    limit: i32,
) -> Result<Vec<Memory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, content, created_at, updated_at, importance, source
             FROM memories
             ORDER BY importance DESC, created_at DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let memories = stmt
        .query_map(params![limit], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                importance: row.get(4)?,
                source: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map memories: {}", e))?;
    
    Ok(memories)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }
}
