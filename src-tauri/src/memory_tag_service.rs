use rusqlite::{Connection, params};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Service module for memory tag operations
/// Handles creation, linking, and retrieval of memory tags

#[derive(Debug, Clone)]
pub struct MemoryTag {
    pub id: String,
    pub tag: String,
    pub color: Option<String>,
    pub created_at: i64,
}

/// Create a new tag
pub fn create_tag(
    conn: &Connection,
    tag_name: String,
    color: Option<String>,
) -> Result<String, String> {
    let tag_id = Uuid::new_v4().to_string();
    let now = current_timestamp();
    
    conn.execute(
        "INSERT INTO memory_tags (id, tag, color, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![tag_id, tag_name, color, now],
    )
    .map_err(|e| format!("Failed to create tag: {}", e))?;
    
    Ok(tag_id)
}

/// Get or create a tag
/// Returns the tag ID (either existing or newly created)
pub fn get_or_create_tag(
    conn: &Connection,
    tag_name: String,
    color: Option<String>,
) -> Result<String, String> {
    // Check if tag already exists
    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM memory_tags WHERE tag = ?1",
        params![tag_name],
        |row| row.get(0),
    );
    
    match existing {
        Ok(id) => Ok(id),
        Err(_) => create_tag(conn, tag_name, color),
    }
}

/// Attach a tag to a memory
pub fn attach_tag_to_memory(
    conn: &Connection,
    memory_id: &str,
    tag_id: &str,
) -> Result<(), String> {
    let now = current_timestamp();
    
    conn.execute(
        "INSERT OR IGNORE INTO memory_tag_links (memory_id, tag_id, created_at)
         VALUES (?1, ?2, ?3)",
        params![memory_id, tag_id, now],
    )
    .map_err(|e| format!("Failed to link tag to memory: {}", e))?;
    
    Ok(())
}

/// Attach a tag to a memory by tag name (creates tag if needed)
pub fn attach_tag_by_name(
    conn: &Connection,
    memory_id: &str,
    tag_name: String,
) -> Result<String, String> {
    let tag_id = get_or_create_tag(conn, tag_name, None)?;
    attach_tag_to_memory(conn, memory_id, &tag_id)?;
    Ok(tag_id)
}

/// Detach a tag from a memory
pub fn detach_tag_from_memory(
    conn: &Connection,
    memory_id: &str,
    tag_id: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM memory_tag_links WHERE memory_id = ?1 AND tag_id = ?2",
        params![memory_id, tag_id],
    )
    .map_err(|e| format!("Failed to unlink tag from memory: {}", e))?;
    
    Ok(())
}

/// Get all tags for a specific memory
pub fn get_memory_tags(
    conn: &Connection,
    memory_id: &str,
) -> Result<Vec<MemoryTag>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT mt.id, mt.tag, mt.color, mt.created_at
             FROM memory_tags mt
             JOIN memory_tag_links mtl ON mt.id = mtl.tag_id
             WHERE mtl.memory_id = ?1
             ORDER BY mt.tag ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let tags = stmt
        .query_map(params![memory_id], |row| {
            Ok(MemoryTag {
                id: row.get(0)?,
                tag: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query tags: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map tags: {}", e))?;
    
    Ok(tags)
}

/// Get all memories with a specific tag
pub fn get_memories_by_tag(
    conn: &Connection,
    tag_name: &str,
    limit: i32,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT m.id
             FROM memories m
             JOIN memory_tag_links mtl ON m.id = mtl.memory_id
             JOIN memory_tags mt ON mtl.tag_id = mt.id
             WHERE mt.tag = ?1
             ORDER BY m.created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let memory_ids = stmt
        .query_map(params![tag_name, limit], |row| row.get(0))
        .map_err(|e| format!("Failed to query memories by tag: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map memory IDs: {}", e))?;
    
    Ok(memory_ids)
}

/// Get all tags
pub fn get_all_tags(conn: &Connection) -> Result<Vec<MemoryTag>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, tag, color, created_at
             FROM memory_tags
             ORDER BY tag ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let tags = stmt
        .query_map([], |row| {
            Ok(MemoryTag {
                id: row.get(0)?,
                tag: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query tags: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map tags: {}", e))?;
    
    Ok(tags)
}

/// Get tag count
pub fn get_tag_count(conn: &Connection) -> Result<i64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_tags",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count tags: {}", e))?;
    
    Ok(count)
}

/// Get count of memories for a tag
pub fn get_tag_memory_count(
    conn: &Connection,
    tag_name: &str,
) -> Result<i64, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_tag_links mtl
             JOIN memory_tags mt ON mtl.tag_id = mt.id
             WHERE mt.tag = ?1",
            params![tag_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count memories for tag: {}", e))?;
    
    Ok(count)
}

/// Delete a tag (will also remove all links)
pub fn delete_tag(
    conn: &Connection,
    tag_id: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM memory_tags WHERE id = ?1",
        params![tag_id],
    )
    .map_err(|e| format!("Failed to delete tag: {}", e))?;
    
    Ok(())
}

/// Delete a tag by name
pub fn delete_tag_by_name(
    conn: &Connection,
    tag_name: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM memory_tags WHERE tag = ?1",
        params![tag_name],
    )
    .map_err(|e| format!("Failed to delete tag: {}", e))?;
    
    Ok(())
}

/// Update tag color
pub fn update_tag_color(
    conn: &Connection,
    tag_id: &str,
    color: Option<String>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE memory_tags SET color = ?1 WHERE id = ?2",
        params![color, tag_id],
    )
    .map_err(|e| format!("Failed to update tag color: {}", e))?;
    
    Ok(())
}

/// Get most frequently used tags
pub fn get_popular_tags(
    conn: &Connection,
    limit: i32,
) -> Result<Vec<(String, i64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT mt.tag, COUNT(*) as count
             FROM memory_tags mt
             JOIN memory_tag_links mtl ON mt.id = mtl.tag_id
             GROUP BY mt.id
             ORDER BY count DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let tags = stmt
        .query_map(params![limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("Failed to query popular tags: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map popular tags: {}", e))?;
    
    Ok(tags)
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
