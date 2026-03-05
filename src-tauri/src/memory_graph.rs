use rusqlite::{Connection, params};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;

/// Service module for automatic memory relationship detection
/// Creates edges between memories based on shared tags and future semantic similarity

#[derive(Debug, Clone)]
pub struct MemoryEdge {
    pub id: String,
    pub source_memory_id: String,
    pub target_memory_id: String,
    pub relationship: String,
    pub weight: f64,
    pub created_at: i64,
}

/// Get current timestamp in seconds since epoch
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Create a memory edge (relationship) between two memories
/// 
/// Avoids duplicates by checking if edge already exists in either direction
/// 
/// # Arguments
/// * `conn` - Database connection
/// * `source_id` - Source memory UUID
/// * `target_id` - Target memory UUID
/// * `relationship` - Type of relationship (e.g., "shared_tag", "semantic_similarity")
/// * `weight` - Numerical weight of the relationship (0.0-1.0)
pub fn create_memory_edge(
    conn: &Connection,
    source_id: String,
    target_id: String,
    relationship: String,
    weight: f64,
) -> Result<String, String> {
    // Prevent self-linking
    if source_id == target_id {
        return Err("Cannot create edge from memory to itself".to_string());
    }

    // Clamp weight to valid range
    let weight = weight.max(0.0).min(1.0);

    // Check if edge already exists in either direction to avoid duplicates
    let existing: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges 
             WHERE (source_memory_id = ?1 AND target_memory_id = ?2)
                OR (source_memory_id = ?2 AND target_memory_id = ?1)
                AND relationship = ?3",
            params![&source_id, &target_id, &relationship],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if existing > 0 {
        return Err(format!(
            "Edge between {} and {} already exists",
            source_id, target_id
        ));
    }

    // Verify both memories exist
    let source_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE id = ?1",
            params![&source_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let target_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE id = ?1",
            params![&target_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if source_exists == 0 || target_exists == 0 {
        return Err("One or both memory IDs do not exist".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();

    conn.execute(
        "INSERT INTO memory_edges (id, source_memory_id, target_memory_id, relationship, weight, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, source_id, target_id, relationship, weight, now],
    )
    .map_err(|e| format!("Failed to create memory edge: {}", e))?;

    Ok(id)
}

/// Generate edges for a specific memory based on shared tags
/// 
/// Finds all other memories that share at least one tag with the given memory,
/// and creates edges with weights proportional to the number of shared tags.
/// 
/// # Arguments
/// * `conn` - Database connection
/// * `memory_id` - UUID of the memory to generate edges for
pub fn generate_edges_for_memory(conn: &Connection, memory_id: String) -> Result<Vec<String>, String> {
    // Get all tags for this memory
    let memory_tags: Vec<String> = conn
        .prepare(
            "SELECT mtl.tag_id FROM memory_tag_links mtl
             WHERE mtl.memory_id = ?1",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?
        .query_map(params![&memory_id], |row| row.get(0))
        .map_err(|e| format!("Failed to query memory tags: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect tags: {}", e))?;

    if memory_tags.is_empty() {
        return Ok(Vec::new());
    }

    let mut created_edges = Vec::new();

    // For each tag, find other memories with that tag and create edges
    let placeholders = vec!["?1"; memory_tags.len()].join(",");
    let query = format!(
        "SELECT DISTINCT mtl2.memory_id, COUNT(*) as shared_count
         FROM memory_tag_links mtl1
         JOIN memory_tag_links mtl2 ON mtl1.tag_id = mtl2.tag_id
         WHERE mtl1.memory_id = ?1
         AND mtl2.memory_id != ?1
         GROUP BY mtl2.memory_id
         ORDER BY shared_count DESC"
    );

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare shared tags query: {}", e))?;

    let related_memories: Vec<(String, i64)> = stmt
        .query_map(params![&memory_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("Failed to query related memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect related memories: {}", e))?;

    // Create edges for memories with shared tags
    let max_tags = memory_tags.len() as f64;

    for (related_id, shared_count) in related_memories {
        // Weight based on proportion of shared tags (0.0-1.0)
        let weight = (shared_count as f64) / max_tags;

        match create_memory_edge(
            conn,
            memory_id.clone(),
            related_id.clone(),
            "shared_tag".to_string(),
            weight,
        ) {
            Ok(edge_id) => {
                created_edges.push(edge_id);
            }
            Err(e) => {
                // Log error but continue with other relationships
                eprintln!("Failed to create edge with {}: {}", related_id, e);
            }
        }
    }

    Ok(created_edges)
}

/// Rebuild the entire memory relationship graph
/// 
/// Clears all existing edges and recreates them based on current tags.
/// This is useful for initial setup or after bulk imports.
/// 
/// Warning: This is computationally expensive and should be run during maintenance windows
pub fn rebuild_memory_graph(conn: &Connection) -> Result<(i64, i64), String> {
    // Get count of existing edges before clearing
    let initial_edge_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges WHERE relationship = 'shared_tag'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Clear existing shared_tag relationships
    conn.execute(
        "DELETE FROM memory_edges WHERE relationship = 'shared_tag'",
        [],
    )
    .map_err(|e| format!("Failed to clear existing edges: {}", e))?;

    // Get all memory IDs
    let memory_ids: Vec<String> = conn
        .prepare("SELECT id FROM memories ORDER BY created_at DESC")
        .map_err(|e| format!("Failed to prepare query: {}", e))?
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to query memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect memory IDs: {}", e))?;

    let mut total_edges_created = 0;

    // Generate edges for each memory
    for memory_id in memory_ids {
        match generate_edges_for_memory(conn, memory_id) {
            Ok(edges) => {
                total_edges_created += edges.len();
            }
            Err(e) => {
                eprintln!("Error generating edges for memory: {}", e);
            }
        }
    }

    Ok((initial_edge_count, total_edges_created as i64))
}

/// Get all edges for a specific memory (both incoming and outgoing)
pub fn get_memory_edges(
    conn: &Connection,
    memory_id: String,
) -> Result<Vec<MemoryEdge>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, source_memory_id, target_memory_id, relationship, weight, created_at 
             FROM memory_edges
             WHERE source_memory_id = ?1 OR target_memory_id = ?1
             ORDER BY weight DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let edges = stmt
        .query_map(params![&memory_id], |row| {
            Ok(MemoryEdge {
                id: row.get(0)?,
                source_memory_id: row.get(1)?,
                target_memory_id: row.get(2)?,
                relationship: row.get(3)?,
                weight: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query edges: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect edges: {}", e))?;

    Ok(edges)
}

/// Get related memories for a memory (following edges with minimum weight threshold)
pub fn get_related_memories(
    conn: &Connection,
    memory_id: String,
    min_weight: f64,
) -> Result<Vec<(String, f64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CASE 
                WHEN source_memory_id = ?1 THEN target_memory_id
                ELSE source_memory_id
             END as related_id, weight
             FROM memory_edges
             WHERE (source_memory_id = ?1 OR target_memory_id = ?1)
             AND weight >= ?2
             AND relationship = 'shared_tag'
             ORDER BY weight DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let related = stmt
        .query_map(params![&memory_id, min_weight], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Failed to query related memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect related memories: {}", e))?;

    Ok(related)
}

/// Delete an edge by ID
pub fn delete_memory_edge(conn: &Connection, edge_id: String) -> Result<(), String> {
    conn.execute("DELETE FROM memory_edges WHERE id = ?1", params![&edge_id])
        .map_err(|e| format!("Failed to delete edge: {}", e))?;
    Ok(())
}

/// Get graph statistics
pub struct GraphStats {
    pub total_edges: i64,
    pub shared_tag_edges: i64,
    pub average_weight: f64,
    pub memories_with_edges: i64,
}

pub fn get_graph_stats(conn: &Connection) -> Result<GraphStats, String> {
    let total_edges: i64 = conn
        .query_row("SELECT COUNT(*) FROM memory_edges", [], |row| row.get(0))
        .unwrap_or(0);

    let shared_tag_edges: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges WHERE relationship = 'shared_tag'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let average_weight: f64 = conn
        .query_row(
            "SELECT AVG(weight) FROM memory_edges",
            [],
            |row| {
                let val: Option<f64> = row.get(0).ok().flatten();
                Ok(val.unwrap_or(0.0))
            },
        )
        .map_err(|e| format!("Failed to get average weight: {}", e))?;

    let memories_with_edges: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT source_memory_id) + COUNT(DISTINCT target_memory_id) FROM memory_edges",
            [],
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

/// Data structure for graph visualization (compatible with react-force-graph)
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

/// Get memory graph data for visualization
/// 
/// Returns top N memories and all edges between them, formatted for react-force-graph
/// This limits the graph to prevent UI overload on large memory collections.
/// 
/// # Arguments
/// * `conn` - Database connection
/// * `limit` - Maximum number of memory nodes to return (default 100)
pub fn get_memory_graph(conn: &Connection, limit: i32) -> Result<MemoryGraphData, String> {
    let limit = limit.max(1).min(1000); // Clamp to reasonable range

    // Get top memories ordered by importance (most important first)
    // Truncate content for readability in graph visualization
    let mut stmt = conn
        .prepare(
            "SELECT id, content, importance FROM memories 
             ORDER BY importance DESC, created_at DESC 
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare memories query: {}", e))?;

    let nodes: Vec<GraphNode> = stmt
        .query_map(params![limit], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let importance: f64 = row.get(2)?;

            // Truncate content to 50 chars for graph labels
            let label = if content.len() > 50 {
                format!("{}...", &content[..47])
            } else {
                content.clone()
            };

            Ok(GraphNode {
                id,
                label,
                importance,
            })
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

    // Get IDs of selected memories for edge filtering
    let memory_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

    // Build parameterized query for edges between selected memories
    // We need to find edges where both source and target are in our selected nodes
    let mut edges = Vec::new();

    for i in 0..memory_ids.len() {
        let current_id = &memory_ids[i];

        // Query edges from this memory
        let mut edge_stmt = conn
            .prepare(
                "SELECT source_memory_id, target_memory_id, weight FROM memory_edges
                 WHERE source_memory_id = ?1 OR target_memory_id = ?1
                 ORDER BY weight DESC",
            )
            .map_err(|e| format!("Failed to prepare edges query: {}", e))?;

        let edge_records: Vec<(String, String, f64)> = edge_stmt
            .query_map(params![current_id], |row| {
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
            // Only include edges where both endpoints are in our selected nodes
            if memory_ids.contains(&source) && memory_ids.contains(&target) {
                // Avoid duplicate edges (both directions)
                let edge_key = if source < target {
                    (source.clone(), target.clone())
                } else {
                    (target.clone(), source.clone())
                };

                // Check if this edge already exists in our list
                let already_exists = edges.iter().any(|e: &GraphEdge| {
                    (e.source == edge_key.0 && e.target == edge_key.1)
                        || (e.source == edge_key.1 && e.target == edge_key.0)
                });

                if !already_exists {
                    edges.push(GraphEdge {
                        source: source.clone(),
                        target: target.clone(),
                        weight,
                    });
                }
            }
        }
    }

    Ok(MemoryGraphData { nodes, edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_clamping() {
        assert_eq!((-0.5_f64).max(0.0).min(1.0), 0.0);
        assert_eq!(1.5_f64.max(0.0).min(1.0), 1.0);
        assert_eq!(0.5_f64.max(0.0).min(1.0), 0.5);
    }

    #[test]
    fn test_timestamp_generation() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }
}
