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
    if let Some(existing_id) = find_near_duplicate_memory(conn, user_id, &content, 0.85)? {
        return Ok(existing_id);
    }

    let now = current_timestamp();
    let uses_integer_id = memories_id_is_integer(conn)?;

    let id = if uses_integer_id {
        conn.execute(
            "INSERT INTO memories (user_id, content, created_at, updated_at, importance, access_count, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![user_id, content, now, now, 0.5, 0, "user_input"],
        )
        .map_err(|e| format!("Failed to create memory: {}", e))?;

        conn.last_insert_rowid().to_string()
    } else {
        let generated_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO memories (id, user_id, content, created_at, updated_at, importance, access_count, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![generated_id, user_id, content, now, now, 0.5, 0, "user_input"],
        )
        .map_err(|e| format!("Failed to create memory: {}", e))?;
        generated_id
    };

    Ok(id)
}

pub fn update_memory_content(
    conn: &Connection,
    user_id: &str,
    memory_id: &str,
    new_content: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE memories SET content = ?1, updated_at = ?2 WHERE id = ?3 AND user_id = ?4",
        params![new_content, current_timestamp(), memory_id, user_id],
    )
    .map_err(|e| format!("Failed to update memory: {}", e))?;
    Ok(())
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
               ORDER BY importance DESC, created_at DESC
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
    let search_pattern = format!("%{}%", query.trim());
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

    if !memories.is_empty() {
        return Ok(memories);
    }

    // Fallback: rank by token overlap so conversational queries still match
    // memories like "I have Big data analytics class tomorrow".
    let tokens = extract_search_tokens(&query);
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let mut candidate_stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT),
                    content,
                    COALESCE(created_at, CAST(strftime('%s', 'now') AS INTEGER)),
                    COALESCE(importance, 0.5),
                    COALESCE(source, 'user_input')
             FROM memories
             WHERE user_id = ?1 AND content IS NOT NULL
             ORDER BY created_at DESC
             LIMIT 250",
        )
        .map_err(|e| format!("Failed to prepare fallback search statement: {}", e))?;

    let candidates = candidate_stmt
        .query_map(params![user_id], |row| {
            Ok(Memory {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
                importance: row.get(3)?,
                source: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query fallback search candidates: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map fallback search candidates: {}", e))?;

    let mut ranked: Vec<(usize, Memory)> = candidates
        .into_iter()
        .filter_map(|memory| {
            let score = overlap_score(&memory.content, &tokens);
            if score > 0 {
                Some((score, memory))
            } else {
                None
            }
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.importance.partial_cmp(&a.1.importance).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| b.1.created_at.cmp(&a.1.created_at))
    });

    Ok(ranked
        .into_iter()
        .take(limit.max(0) as usize)
        .map(|(_, memory)| memory)
        .collect())
}

fn extract_search_tokens(query: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "and", "are", "about", "can", "do", "does", "for", "have", "hey", "i",
        "in", "is", "know", "me", "my", "of", "on", "or", "please", "that", "the", "to",
        "what", "when", "where", "who", "why", "you",
    ];

    let normalized_query = query.to_lowercase();

    let mut seen = std::collections::HashSet::new();
    let mut tokens = normalized_query
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|token| token.len() >= 2)
        .filter(|token| !STOP_WORDS.contains(token))
        .filter(|token| seen.insert((*token).to_string()))
        .map(|token| token.to_string())
        .collect::<Vec<_>>();

    // Temporal expansion: "start of the week" should match Monday memories.
    if (normalized_query.contains("start of the week")
        || normalized_query.contains("start of week")
        || normalized_query.contains("beginning of the week")
        || normalized_query.contains("beginning of week"))
        && seen.insert("monday".to_string())
    {
        tokens.push("monday".to_string());
    }

    tokens
}

fn overlap_score(content: &str, query_tokens: &[String]) -> usize {
    let normalized = content
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>();

    query_tokens
        .iter()
        .filter(|token| normalized.contains(token.as_str()))
        .count()
}

fn find_near_duplicate_memory(
    conn: &Connection,
    user_id: &str,
    incoming: &str,
    threshold: f64,
) -> Result<Option<String>, String> {
    let normalized_incoming = normalize_for_similarity(incoming);
    if normalized_incoming.is_empty() {
        return Ok(None);
    }

    let mut stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT), content
             FROM memories
             WHERE user_id = ?1 AND content IS NOT NULL
             ORDER BY created_at DESC
             LIMIT 300",
        )
        .map_err(|e| format!("Failed to prepare duplicate search statement: {}", e))?;

    let mut rows = stmt
        .query(params![user_id])
        .map_err(|e| format!("Failed to query duplicate search rows: {}", e))?;

    while let Some(row) = rows
        .next()
        .map_err(|e| format!("Failed to iterate duplicate search rows: {}", e))?
    {
        let id: String = row.get(0).map_err(|e| format!("Failed to read memory id: {}", e))?;
        let content: String = row
            .get(1)
            .map_err(|e| format!("Failed to read memory content: {}", e))?;
        let similarity = text_similarity(&normalized_incoming, &normalize_for_similarity(&content));
        if similarity >= threshold {
            return Ok(Some(id));
        }
    }

    Ok(None)
}

fn normalize_for_similarity(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn text_similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }

    let left_tokens = left.split_whitespace().collect::<std::collections::HashSet<_>>();
    let right_tokens = right
        .split_whitespace()
        .collect::<std::collections::HashSet<_>>();

    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let intersection = left_tokens.intersection(&right_tokens).count() as f64;
    let union = left_tokens.union(&right_tokens).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
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
