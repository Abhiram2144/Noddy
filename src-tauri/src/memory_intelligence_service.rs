use crate::memory_graph_repository::{
    create_edge, delete_relationships_by_type, delete_relationships_for_memory, get_edges_for_memory,
    get_memory, list_all_memories, list_candidate_memories, list_edges, list_memories,
    record_memory_access, update_memory_importance, MemoryEdgeRecord,
};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

const KEYWORD_RELATIONSHIP: &str = "keyword_similarity";
const SIMILARITY_THRESHOLD: f64 = 0.22;
const CANDIDATE_LIMIT: i32 = 250;

#[derive(Debug, Clone, Serialize)]
pub struct RelatedMemory {
    pub memory_id: String,
    pub weight: f64,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub content: String,
    pub importance: f64,
    pub cluster_id: String,
    pub connection_count: usize,
    pub access_count: i64,
    pub recentness: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_edges: i64,
    pub keyword_edges: i64,
    pub average_weight: f64,
    pub memories_with_edges: i64,
    pub clusters: i64,
}

pub fn calculate_memory_importance(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
) -> Result<f64, String> {
    let memory = get_memory(conn, user_id, memory_id)?;
    let edge_count = get_edges_for_memory(conn, user_id, memory_id)?.len();
    let score = compute_importance(
        memory.created_at,
        memory.access_count,
        edge_count,
        current_timestamp(),
    );
    update_memory_importance(conn, user_id, memory_id, score)?;
    Ok(score)
}

pub fn link_related_memories(
    conn: &Connection,
    user_id: &str,
    new_memory_id: &str,
) -> Result<usize, String> {
    let memory = get_memory(conn, user_id, new_memory_id)?;
    let source_tokens = tokenize(&memory.content);

    delete_relationships_for_memory(conn, user_id, new_memory_id, KEYWORD_RELATIONSHIP)?;

    if source_tokens.is_empty() {
        calculate_memory_importance(conn, user_id, new_memory_id)?;
        return Ok(0);
    }

    let candidates = list_candidate_memories(conn, user_id, new_memory_id, CANDIDATE_LIMIT)?;
    let mut linked_ids = Vec::new();
    let mut created_edges = 0;

    for candidate in candidates {
        let target_tokens = tokenize(&candidate.content);
        if target_tokens.is_empty() {
            continue;
        }

        let similarity = keyword_similarity(&source_tokens, &target_tokens);
        if similarity >= SIMILARITY_THRESHOLD {
            let result = create_edge(
                conn,
                user_id,
                new_memory_id,
                &candidate.id,
                KEYWORD_RELATIONSHIP,
                similarity,
            )?;

            if result.created {
                created_edges += 1;
            }

            linked_ids.push(candidate.id);
        }
    }

    let mut impacted_ids: HashSet<String> = linked_ids.into_iter().collect();
    impacted_ids.insert(new_memory_id.to_string());
    refresh_importance_for_memories(conn, user_id, impacted_ids.into_iter())?;

    Ok(created_edges)
}

pub fn rebuild_memory_links(conn: &Connection, user_id: &str) -> Result<(i64, i64), String> {
    let existing_edges = list_edges(conn, user_id)?;
    let cleared_edges = existing_edges
        .iter()
        .filter(|edge| edge.relationship == KEYWORD_RELATIONSHIP)
        .count() as i64;

    delete_relationships_by_type(conn, user_id, KEYWORD_RELATIONSHIP)?;

    let memories = list_all_memories(conn, user_id)?;
    let tokenized: Vec<(String, HashSet<String>)> = memories
        .iter()
        .map(|memory| (memory.id.clone(), tokenize(&memory.content)))
        .collect();

    let mut created_edges = 0_i64;
    for index in 0..tokenized.len() {
        for other_index in (index + 1)..tokenized.len() {
            let (left_id, left_tokens) = &tokenized[index];
            let (right_id, right_tokens) = &tokenized[other_index];
            if left_tokens.is_empty() || right_tokens.is_empty() {
                continue;
            }

            let similarity = keyword_similarity(left_tokens, right_tokens);
            if similarity >= SIMILARITY_THRESHOLD {
                let result = create_edge(
                    conn,
                    user_id,
                    left_id,
                    right_id,
                    KEYWORD_RELATIONSHIP,
                    similarity,
                )?;

                if result.created {
                    created_edges += 1;
                }
            }
        }
    }

    refresh_importance_for_memories(conn, user_id, memories.into_iter().map(|memory| memory.id))?;
    Ok((cleared_edges, created_edges))
}

pub fn get_related_memories(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
    min_weight: f64,
) -> Result<Vec<RelatedMemory>, String> {
    let mut related = get_edges_for_memory(conn, user_id, memory_id)?
        .into_iter()
        .filter(|edge| edge.weight >= min_weight)
        .map(|edge| RelatedMemory {
            memory_id: if edge.source_memory_id == memory_id {
                edge.target_memory_id
            } else {
                edge.source_memory_id
            },
            weight: edge.weight,
            relationship: edge.relationship,
        })
        .collect::<Vec<_>>();

    related.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(related)
}

pub fn get_graph_data(conn: &Connection, user_id: &str, limit: i32) -> Result<GraphData, String> {
    let limit = limit.max(1).min(1000);
    let nodes = list_memories(conn, user_id, limit)?;

    if nodes.is_empty() {
        return Ok(GraphData {
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    }

    let node_ids: HashSet<String> = nodes.iter().map(|node| node.id.clone()).collect();
    let all_edges = list_edges(conn, user_id)?;
    let edges = all_edges
        .into_iter()
        .filter(|edge| node_ids.contains(&edge.source_memory_id) && node_ids.contains(&edge.target_memory_id))
        .collect::<Vec<_>>();

    let connection_counts = build_connection_counts(&edges);
    let clusters = build_clusters(&node_ids, &edges);
    let now = current_timestamp();

    let graph_nodes = nodes
        .into_iter()
        .map(|memory| GraphNode {
            id: memory.id.clone(),
            label: truncate_label(&memory.content),
            content: memory.content,
            importance: memory.importance,
            cluster_id: clusters
                .get(&memory.id)
                .cloned()
                .unwrap_or_else(|| "cluster-0".to_string()),
            connection_count: *connection_counts.get(&memory.id).unwrap_or(&0),
            access_count: memory.access_count,
            recentness: compute_recency_score(memory.created_at, now),
        })
        .collect::<Vec<_>>();

    let graph_edges = edges
        .into_iter()
        .map(|edge| GraphEdge {
            source: edge.source_memory_id,
            target: edge.target_memory_id,
            weight: edge.weight,
            relationship: edge.relationship,
        })
        .collect();

    Ok(GraphData {
        nodes: graph_nodes,
        edges: graph_edges,
    })
}

pub fn get_graph_stats(conn: &Connection, user_id: &str) -> Result<GraphStats, String> {
    let edges = list_edges(conn, user_id)?;
    let memories_with_edges = build_connection_counts(&edges).len() as i64;
    let average_weight = if edges.is_empty() {
        0.0
    } else {
        edges.iter().map(|edge| edge.weight).sum::<f64>() / edges.len() as f64
    };

    let node_ids: HashSet<String> = list_all_memories(conn, user_id)?
        .into_iter()
        .map(|memory| memory.id)
        .collect();
    let clusters = build_clusters(&node_ids, &edges).values().collect::<HashSet<_>>().len() as i64;

    Ok(GraphStats {
        total_edges: edges.len() as i64,
        keyword_edges: edges
            .iter()
            .filter(|edge| edge.relationship == KEYWORD_RELATIONSHIP)
            .count() as i64,
        average_weight,
        memories_with_edges,
        clusters,
    })
}

pub fn record_access_and_refresh(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
) -> Result<f64, String> {
    record_memory_access(conn, user_id, memory_id)?;

    let mut impacted_ids = HashSet::new();
    impacted_ids.insert(memory_id.to_string());
    for edge in get_edges_for_memory(conn, user_id, memory_id)? {
        impacted_ids.insert(edge.source_memory_id);
        impacted_ids.insert(edge.target_memory_id);
    }

    let mut latest_score = 0.5;
    for impacted_id in impacted_ids {
        let score = calculate_memory_importance(conn, user_id, &impacted_id)?;
        if impacted_id == memory_id {
            latest_score = score;
        }
    }

    Ok(latest_score)
}

fn refresh_importance_for_memories<I>(
    conn: &Connection,
    user_id: &str,
    memory_ids: I,
) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let unique_ids = memory_ids.into_iter().collect::<HashSet<_>>();
    for memory_id in unique_ids {
        calculate_memory_importance(conn, user_id, &memory_id)?;
    }
    Ok(())
}

fn compute_importance(created_at: i64, access_count: i64, connection_count: usize, now: i64) -> f64 {
    let recency = compute_recency_score(created_at, now);
    let access = 1.0 - (-(access_count.max(0) as f64) / 4.0).exp();
    let connectivity = 1.0 - (-(connection_count as f64) / 3.0).exp();
    (0.45 * recency + 0.25 * access + 0.30 * connectivity)
        .clamp(0.05, 1.0)
}

fn compute_recency_score(created_at: i64, now: i64) -> f64 {
    let age_days = ((now - created_at).max(0) as f64) / 86_400.0;
    let lambda = 0.08_f64;
    (-lambda * age_days).exp()
}

fn tokenize(content: &str) -> HashSet<String> {
    let stopwords: HashSet<&'static str> = [
        "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "i", "in",
        "is", "it", "me", "my", "of", "on", "or", "our", "that", "the", "their", "this", "to",
        "was", "we", "with", "you", "your",
    ]
    .into_iter()
    .collect();

    content
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|token| {
            let normalized = token.trim().to_lowercase();
            if normalized.len() < 3 || stopwords.contains(normalized.as_str()) {
                None
            } else {
                Some(normalized)
            }
        })
        .collect()
}

fn keyword_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn build_connection_counts(edges: &[MemoryEdgeRecord]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for edge in edges {
        *counts.entry(edge.source_memory_id.clone()).or_insert(0) += 1;
        *counts.entry(edge.target_memory_id.clone()).or_insert(0) += 1;
    }
    counts
}

fn build_clusters(
    node_ids: &HashSet<String>,
    edges: &[MemoryEdgeRecord],
) -> HashMap<String, String> {
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for node_id in node_ids {
        adjacency.entry(node_id.clone()).or_default();
    }

    for edge in edges {
        adjacency
            .entry(edge.source_memory_id.clone())
            .or_default()
            .push(edge.target_memory_id.clone());
        adjacency
            .entry(edge.target_memory_id.clone())
            .or_default()
            .push(edge.source_memory_id.clone());
    }

    let mut visited = HashSet::new();
    let mut clusters = HashMap::new();
    let mut cluster_index = 1;

    for node_id in node_ids {
        if visited.contains(node_id) {
            continue;
        }

        let cluster_id = format!("cluster-{}", cluster_index);
        cluster_index += 1;

        let mut stack = vec![node_id.clone()];
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            clusters.insert(current.clone(), cluster_id.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }
    }

    clusters
}

fn truncate_label(content: &str) -> String {
    if content.chars().count() > 64 {
        let truncated = content.chars().take(61).collect::<String>();
        format!("{}...", truncated)
    } else {
        content.to_string()
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}