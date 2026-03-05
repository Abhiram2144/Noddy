use rusqlite::{params, Connection};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn create_memory_edge(
    conn: &Connection,
    user_id: &str,
    source_id: String,
    target_id: String,
    relationship: String,
    weight: f64,
) -> Result<String, String> {
    if source_id == target_id {
        return Err("Cannot create edge from memory to itself".to_string());
    }

    let weight = weight.clamp(0.0, 1.0);

    let existing: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges
             WHERE user_id = ?1
               AND relationship = ?2
               AND ((source_memory_id = ?3 AND target_memory_id = ?4)
                 OR (source_memory_id = ?4 AND target_memory_id = ?3))",
            params![user_id, &relationship, &source_id, &target_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if existing > 0 {
        return Err(format!("Edge between {} and {} already exists", source_id, target_id));
    }

    let source_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE CAST(id AS TEXT) = ?1 AND user_id = ?2",
            params![&source_id, user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let target_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE CAST(id AS TEXT) = ?1 AND user_id = ?2",
            params![&target_id, user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if source_exists == 0 || target_exists == 0 {
        return Err("One or both memory IDs do not exist for this user".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();

    conn.execute(
        "INSERT INTO memory_edges (id, user_id, source_memory_id, target_memory_id, relationship, weight, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, user_id, source_id, target_id, relationship, weight, now],
    )
    .map_err(|e| format!("Failed to create memory edge: {}", e))?;

    Ok(id)
}

pub fn generate_edges_for_memory(conn: &Connection, user_id: &str, memory_id: String) -> Result<Vec<String>, String> {
    let memory_tags: Vec<String> = conn
        .prepare(
            "SELECT mtl.tag_id FROM memory_tag_links mtl
             JOIN memories m ON m.id = mtl.memory_id
             WHERE mtl.memory_id = ?1 AND m.user_id = ?2",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?
        .query_map(params![&memory_id, user_id], |row| row.get(0))
        .map_err(|e| format!("Failed to query memory tags: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect tags: {}", e))?;

    if memory_tags.is_empty() {
        return Ok(Vec::new());
    }

    let query =
        "SELECT DISTINCT mtl2.memory_id, COUNT(*) as shared_count
         FROM memory_tag_links mtl1
         JOIN memory_tag_links mtl2 ON mtl1.tag_id = mtl2.tag_id
         JOIN memories m2 ON m2.id = mtl2.memory_id
         WHERE mtl1.memory_id = ?1
           AND mtl2.memory_id != ?1
           AND m2.user_id = ?2
         GROUP BY mtl2.memory_id
         ORDER BY shared_count DESC";

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare shared tags query: {}", e))?;

    let related_memories: Vec<(String, i64)> = stmt
        .query_map(params![&memory_id, user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("Failed to query related memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect related memories: {}", e))?;

    let mut created_edges = Vec::new();
    let max_tags = memory_tags.len() as f64;

    for (related_id, shared_count) in related_memories {
        let weight = (shared_count as f64) / max_tags;
        match create_memory_edge(
            conn,
            user_id,
            memory_id.clone(),
            related_id.clone(),
            "shared_tag".to_string(),
            weight,
        ) {
            Ok(edge_id) => created_edges.push(edge_id),
            Err(e) => eprintln!("Failed to create edge with {}: {}", related_id, e),
        }
    }

    Ok(created_edges)
}

pub fn rebuild_memory_graph(conn: &Connection, user_id: &str) -> Result<(i64, i64), String> {
    let initial_edge_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges WHERE user_id = ?1 AND relationship = 'shared_tag'",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        "DELETE FROM memory_edges WHERE user_id = ?1 AND relationship = 'shared_tag'",
        params![user_id],
    )
    .map_err(|e| format!("Failed to clear existing edges: {}", e))?;

    let memory_ids: Vec<String> = conn
        .prepare("SELECT CAST(id AS TEXT) FROM memories WHERE user_id = ?1 ORDER BY created_at DESC")
        .map_err(|e| format!("Failed to prepare query: {}", e))?
        .query_map(params![user_id], |row| row.get(0))
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect memory IDs: {}", e))?;

    let mut total_edges_created = 0;
    for memory_id in memory_ids {
        match generate_edges_for_memory(conn, user_id, memory_id) {
            Ok(edges) => total_edges_created += edges.len(),
            Err(e) => eprintln!("Error generating edges for memory: {}", e),
        }
    }

    Ok((initial_edge_count, total_edges_created as i64))
}

pub fn get_related_memories(
    conn: &Connection,
    user_id: &str,
    memory_id: String,
    min_weight: f64,
) -> Result<Vec<(String, f64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CASE
                WHEN source_memory_id = ?2 THEN target_memory_id
                ELSE source_memory_id
             END as related_id, weight
             FROM memory_edges
             WHERE user_id = ?1
               AND (source_memory_id = ?2 OR target_memory_id = ?2)
               AND weight >= ?3
               AND relationship = 'shared_tag'
             ORDER BY weight DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let related = stmt
        .query_map(params![user_id, &memory_id, min_weight], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Failed to query related memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect related memories: {}", e))?;

    Ok(related)
}

pub struct GraphStats {
    pub total_edges: i64,
    pub shared_tag_edges: i64,
    pub average_weight: f64,
    pub memories_with_edges: i64,
}

pub fn get_graph_stats(conn: &Connection, user_id: &str) -> Result<GraphStats, String> {
    let total_edges: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let shared_tag_edges: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges WHERE user_id = ?1 AND relationship = 'shared_tag'",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let average_weight: f64 = conn
        .query_row(
            "SELECT AVG(weight) FROM memory_edges WHERE user_id = ?1",
            params![user_id],
            |row| {
                let val: Option<f64> = row.get(0).ok().flatten();
                Ok(val.unwrap_or(0.0))
            },
        )
        .map_err(|e| format!("Failed to get average weight: {}", e))?;

    let memories_with_edges: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT source_memory_id) + COUNT(DISTINCT target_memory_id)
             FROM memory_edges WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(GraphStats {
        total_edges,
        shared_tag_edges,
        average_weight,
        memories_with_edges,
    })
}

#[derive(Debug, Clone)]
pub struct MemoryGraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub importance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
}

pub fn get_memory_graph(conn: &Connection, user_id: &str, limit: i32) -> Result<MemoryGraphData, String> {
    let limit = limit.max(1).min(1000);

    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(importance, 0.5)
             FROM memories
             WHERE user_id = ?1
             ORDER BY importance DESC, created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare memories query: {}", e))?;

    let nodes: Vec<GraphNode> = stmt
        .query_map(params![user_id, limit], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let importance: f64 = row.get(2)?;

            let label = if content.len() > 50 {
                format!("{}...", &content[..47])
            } else {
                content
            };

            Ok(GraphNode { id, label, importance })
        })
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect nodes: {}", e))?;

    if nodes.is_empty() {
        return Ok(MemoryGraphData {
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    }

    let memory_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let mut edges = Vec::new();

    for current_id in &memory_ids {
        let mut edge_stmt = conn
            .prepare(
                "SELECT source_memory_id, target_memory_id, weight FROM memory_edges
                 WHERE user_id = ?1 AND (source_memory_id = ?2 OR target_memory_id = ?2)
                 ORDER BY weight DESC",
            )
            .map_err(|e| format!("Failed to prepare edges query: {}", e))?;

        let edge_records: Vec<(String, String, f64)> = edge_stmt
            .query_map(params![user_id, current_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })
            .map_err(|e| format!("Failed to query edges: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect edges: {}", e))?;

        for (source, target, weight) in edge_records {
            if memory_ids.contains(&source) && memory_ids.contains(&target) {
                let edge_key = if source < target {
                    (source.clone(), target.clone())
                } else {
                    (target.clone(), source.clone())
                };

                let already_exists = edges.iter().any(|e: &GraphEdge| {
                    (e.source == edge_key.0 && e.target == edge_key.1)
                        || (e.source == edge_key.1 && e.target == edge_key.0)
                });

                if !already_exists {
                    edges.push(GraphEdge { source, target, weight });
                }
            }
        }
    }

    Ok(MemoryGraphData { nodes, edges })
}
