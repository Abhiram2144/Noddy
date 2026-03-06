use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub id: String,
    pub content: String,
    pub created_at: i64,
    pub importance: f64,
    pub access_count: i64,
}

#[derive(Debug, Clone)]
pub struct MemoryEdgeRecord {
    pub source_memory_id: String,
    pub target_memory_id: String,
    pub relationship: String,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct EdgeWriteResult {
    pub created: bool,
}

pub fn get_memory(conn: &Connection, user_id: &str, memory_id: &str) -> Result<MemoryRecord, String> {
    conn.query_row(
        "SELECT CAST(id AS TEXT),
                content,
                COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                COALESCE(importance, 0.5),
                COALESCE(access_count, 0),
                last_accessed_at
         FROM memories
         WHERE user_id = ?1 AND CAST(id AS TEXT) = ?2",
        params![user_id, memory_id],
        |row| {
            Ok(MemoryRecord {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                importance: row.get(3)?,
                access_count: row.get(4)?,
            })
        },
    )
    .map_err(|e| format!("Failed to fetch memory {}: {}", memory_id, e))
}

pub fn list_memories(conn: &Connection, user_id: &str, limit: i32) -> Result<Vec<MemoryRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                    COALESCE(importance, 0.5),
                    COALESCE(access_count, 0),
                    last_accessed_at
             FROM memories
             WHERE user_id = ?1 AND content IS NOT NULL
             ORDER BY importance DESC, created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare memories query: {}", e))?;

    let rows = stmt.query_map(params![user_id, limit], |row| {
        Ok(MemoryRecord {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
            importance: row.get(3)?,
            access_count: row.get(4)?,
        })
    })
    .map_err(|e| format!("Failed to query memories: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect memories: {}", e))
}

pub fn list_all_memories(conn: &Connection, user_id: &str) -> Result<Vec<MemoryRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                    COALESCE(importance, 0.5),
                    COALESCE(access_count, 0),
                    last_accessed_at
             FROM memories
             WHERE user_id = ?1 AND content IS NOT NULL
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare all memories query: {}", e))?;

    let rows = stmt.query_map(params![user_id], |row| {
        Ok(MemoryRecord {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
            importance: row.get(3)?,
            access_count: row.get(4)?,
        })
    })
    .map_err(|e| format!("Failed to query all memories: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect all memories: {}", e))
}

pub fn list_candidate_memories(
    conn: &Connection,
    user_id: &str,
    exclude_memory_id: &str,
    limit: i32,
) -> Result<Vec<MemoryRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                    COALESCE(importance, 0.5),
                    COALESCE(access_count, 0),
                    last_accessed_at
             FROM memories
             WHERE user_id = ?1
               AND CAST(id AS TEXT) != ?2
               AND content IS NOT NULL
             ORDER BY created_at DESC
             LIMIT ?3",
        )
        .map_err(|e| format!("Failed to prepare candidate memories query: {}", e))?;

    let rows = stmt.query_map(params![user_id, exclude_memory_id, limit], |row| {
        Ok(MemoryRecord {
            id: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
            importance: row.get(3)?,
            access_count: row.get(4)?,
        })
    })
    .map_err(|e| format!("Failed to query candidate memories: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect candidate memories: {}", e))
}

pub fn create_edge(
    conn: &Connection,
    user_id: &str,
    source_id: &str,
    target_id: &str,
    relationship: &str,
    weight: f64,
) -> Result<EdgeWriteResult, String> {
    if source_id == target_id {
        return Err("Cannot create edge from memory to itself".to_string());
    }

    let (source_memory_id, target_memory_id) = canonicalize_pair(source_id, target_id);
    let weight = weight.clamp(0.0, 1.0);
    let now = current_timestamp();

    let existing: Result<String, _> = conn.query_row(
        "SELECT id FROM memory_edges
         WHERE user_id = ?1
           AND relationship = ?2
           AND source_memory_id = ?3
           AND target_memory_id = ?4",
        params![user_id, relationship, &source_memory_id, &target_memory_id],
        |row| row.get(0),
    );

    match existing {
        Ok(id) => {
            conn.execute(
                "UPDATE memory_edges
                 SET weight = ?1, created_at = ?2
                 WHERE id = ?3",
                params![weight, now, &id],
            )
            .map_err(|e| format!("Failed to update memory edge: {}", e))?;

            Ok(EdgeWriteResult { created: false })
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO memory_edges (id, user_id, source_memory_id, target_memory_id, relationship, weight, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, user_id, source_memory_id, target_memory_id, relationship, weight, now],
            )
            .map_err(|e| format!("Failed to create memory edge: {}", e))?;

            Ok(EdgeWriteResult { created: true })
        }
        Err(e) => Err(format!("Failed to query existing memory edge: {}", e)),
    }
}

pub fn get_edges_for_memory(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
) -> Result<Vec<MemoryEdgeRecord>, String> {
    let mut stmt = conn
        .prepare(
                        "SELECT source_memory_id, target_memory_id, COALESCE(relationship, 'related'), COALESCE(weight, 0.0)
             FROM memory_edges
             WHERE user_id = ?1
               AND (source_memory_id = ?2 OR target_memory_id = ?2)
             ORDER BY weight DESC, created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare edge query: {}", e))?;

    let rows = stmt.query_map(params![user_id, memory_id], |row| {
        Ok(MemoryEdgeRecord {
            source_memory_id: row.get(0)?,
            target_memory_id: row.get(1)?,
            relationship: row.get(2)?,
            weight: row.get(3)?,
        })
    })
    .map_err(|e| format!("Failed to query edges: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect edges: {}", e))
}

pub fn list_edges(conn: &Connection, user_id: &str) -> Result<Vec<MemoryEdgeRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT source_memory_id, target_memory_id, COALESCE(relationship, 'related'), COALESCE(weight, 0.0)
             FROM memory_edges
             WHERE user_id = ?1
             ORDER BY weight DESC, created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare list edges query: {}", e))?;

    let rows = stmt.query_map(params![user_id], |row| {
        Ok(MemoryEdgeRecord {
            source_memory_id: row.get(0)?,
            target_memory_id: row.get(1)?,
            relationship: row.get(2)?,
            weight: row.get(3)?,
        })
    })
    .map_err(|e| format!("Failed to query all edges: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect all edges: {}", e))
}

pub fn delete_relationships_for_memory(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
    relationship: &str,
) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM memory_edges
         WHERE user_id = ?1
           AND relationship = ?2
           AND (source_memory_id = ?3 OR target_memory_id = ?3)",
        params![user_id, relationship, memory_id],
    )
    .map_err(|e| format!("Failed to delete memory relationships: {}", e))
}

pub fn delete_relationships_by_type(
    conn: &Connection,
    user_id: &str,
    relationship: &str,
) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM memory_edges WHERE user_id = ?1 AND relationship = ?2",
        params![user_id, relationship],
    )
    .map_err(|e| format!("Failed to clear relationships: {}", e))
}

pub fn update_memory_importance(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
    importance: f64,
) -> Result<(), String> {
    conn.execute(
        "UPDATE memories
         SET importance = ?1, updated_at = ?2
         WHERE user_id = ?3 AND CAST(id AS TEXT) = ?4",
        params![importance.clamp(0.0, 1.0), current_timestamp(), user_id, memory_id],
    )
    .map_err(|e| format!("Failed to update memory importance: {}", e))?;

    Ok(())
}

pub fn record_memory_access(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
) -> Result<(i64, i64), String> {
    let now = current_timestamp();
    conn.execute(
        "UPDATE memories
         SET access_count = COALESCE(access_count, 0) + 1,
             last_accessed_at = ?1,
             updated_at = ?1
         WHERE user_id = ?2 AND CAST(id AS TEXT) = ?3",
        params![now, user_id, memory_id],
    )
    .map_err(|e| format!("Failed to record memory access: {}", e))?;

    let access_count: i64 = conn
        .query_row(
            "SELECT COALESCE(access_count, 0) FROM memories WHERE user_id = ?1 AND CAST(id AS TEXT) = ?2",
            params![user_id, memory_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to fetch updated access count: {}", e))?;

    Ok((access_count, now))
}

fn canonicalize_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}