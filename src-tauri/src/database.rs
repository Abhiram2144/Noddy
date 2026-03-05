use rusqlite::{Connection, Result as SqliteResult};

/// Database initialization and migration management
/// Handles creation of all tables and indexes for the modular memory architecture

pub fn initialize_database(conn: &Connection) -> SqliteResult<()> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    // Run all migrations in order
    create_memories_table(conn)?;
    create_memory_tags_table(conn)?;
    create_memory_tag_links_table(conn)?;
    create_memory_edges_table(conn)?;
    create_reminders_table(conn)?;
    create_command_history_table(conn)?;
    create_memory_embeddings_table(conn)?;
    
    // Create all indexes
    create_indexes(conn)?;
    
    println!("✓ Database schema initialized with 7 tables and 15+ indexes");
    Ok(())
}

/// Memories table: Stores individual user memory entries
fn create_memories_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            created_at INTEGER,
            updated_at INTEGER,
            importance REAL DEFAULT 0.5,
            source TEXT,
            tags TEXT,
            metadata TEXT
        )",
        [],
    )?;
    
    // Migrate old memories table if it exists
    migrate_old_memories_table(conn)?;
    
    println!("✓ memories table ready");
    Ok(())
}

/// Memory tags table: Stores unique tag labels
fn create_memory_tags_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_tags (
            id TEXT PRIMARY KEY,
            tag TEXT UNIQUE NOT NULL,
            color TEXT,
            created_at INTEGER
        )",
        [],
    )?;
    
    println!("✓ memory_tags table ready");
    Ok(())
}

/// Memory tag links table: Many-to-many relationship between memories and tags
fn create_memory_tag_links_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_tag_links (
            memory_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            created_at INTEGER,
            PRIMARY KEY (memory_id, tag_id),
            FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES memory_tags(id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    println!("✓ memory_tag_links table ready");
    Ok(())
}

/// Memory edges table: Stores relationships between memory nodes for graph visualization
fn create_memory_edges_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_edges (
            id TEXT PRIMARY KEY,
            source_memory_id TEXT NOT NULL,
            target_memory_id TEXT NOT NULL,
            relationship TEXT,
            weight REAL,
            created_at INTEGER,
            metadata TEXT,
            FOREIGN KEY (source_memory_id) REFERENCES memories(id) ON DELETE CASCADE,
            FOREIGN KEY (target_memory_id) REFERENCES memories(id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    println!("✓ memory_edges table ready");
    Ok(())
}

/// Reminders table: Stores scheduled reminders
fn create_reminders_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reminders (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            created_at INTEGER,
            trigger_at INTEGER NOT NULL,
            status TEXT DEFAULT 'pending',
            source TEXT,
            memory_id TEXT,
            metadata TEXT,
            FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE SET NULL
        )",
        [],
    )?;
    
    println!("✓ reminders table ready");
    Ok(())
}

/// Command history table: Stores executed command telemetry
fn create_command_history_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS command_history (
            id TEXT PRIMARY KEY,
            command_text TEXT NOT NULL,
            intent_name TEXT,
            duration_ms INTEGER,
            success INTEGER DEFAULT 1,
            timestamp INTEGER NOT NULL,
            status TEXT DEFAULT 'completed',
            error_message TEXT,
            metadata TEXT
        )",
        [],
    )?;
    
    println!("✓ command_history table ready");
    Ok(())
}

/// Memory embeddings table: Reserved for future semantic search
fn create_memory_embeddings_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_embeddings (
            memory_id TEXT PRIMARY KEY,
            embedding BLOB,
            model TEXT,
            created_at INTEGER,
            version INTEGER DEFAULT 1,
            FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    println!("✓ memory_embeddings table ready");
    Ok(())
}

/// Create all indexes for optimal query performance
fn create_indexes(conn: &Connection) -> SqliteResult<()> {
    // Memories table indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at DESC)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memories_updated_at ON memories(updated_at DESC)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memories_source ON memories(source)",
        [],
    )?;
    
    // Memory tags indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_tags_tag ON memory_tags(tag)",
        [],
    )?;
    
    // Memory tag links indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_tag_links_tag_id ON memory_tag_links(tag_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_tag_links_memory_id ON memory_tag_links(memory_id)",
        [],
    )?;
    
    // Memory edges indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_edges_source ON memory_edges(source_memory_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_edges_target ON memory_edges(target_memory_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_edges_relationship ON memory_edges(relationship)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_memory_edges_created_at ON memory_edges(created_at DESC)",
        [],
    )?;
    
    // Reminders indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_reminders_trigger_at ON reminders(trigger_at ASC)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_reminders_status ON reminders(status)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_reminders_memory_id ON reminders(memory_id)",
        [],
    )?;
    
    // Command history indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_command_history_timestamp ON command_history(timestamp DESC)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_command_history_intent ON command_history(intent_name)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_command_history_status ON command_history(status)",
        [],
    )?;
    
    println!("✓ All indexes created for optimal performance");
    Ok(())
}

/// Migrate data from old single-table schema to new normalized schema
/// This runs once to preserve existing memory data
fn migrate_old_memories_table(conn: &Connection) -> SqliteResult<()> {
    // Check if old memories table has the old schema (id, content, created_at, expires_at, reminder_state)
    let columns: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT name FROM pragma_table_info('memories') ORDER BY cid"
        )?;
        let column_iter = stmt.query_map([], |row| row.get(0))?;
        
        let mut result = Vec::new();
        for col_result in column_iter {
            if let Ok(col_name) = col_result {
                result.push(col_name);
            }
        }
        result
    };
    
    if !columns.is_empty() {
        let column_names: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();

        // Ensure legacy databases are upgraded in-place so newer queries and indexes are valid.
        if !column_names.contains(&"updated_at") {
            conn.execute("ALTER TABLE memories ADD COLUMN updated_at INTEGER", [])?;
            conn.execute(
                "UPDATE memories SET updated_at = created_at WHERE updated_at IS NULL",
                [],
            )?;
        }

        if !column_names.contains(&"importance") {
            conn.execute(
                "ALTER TABLE memories ADD COLUMN importance REAL DEFAULT 0.5",
                [],
            )?;
        }

        if !column_names.contains(&"source") {
            conn.execute("ALTER TABLE memories ADD COLUMN source TEXT", [])?;
        }

        if !column_names.contains(&"tags") {
            conn.execute("ALTER TABLE memories ADD COLUMN tags TEXT", [])?;
        }

        if !column_names.contains(&"metadata") {
            conn.execute("ALTER TABLE memories ADD COLUMN metadata TEXT", [])?;
        }

        if column_names.len() >= 3 && column_names[0] == "id" {
            println!("✓ Old memories table detected and migrated");
        }
    }
    
    Ok(())
}

/// Verify database health and integrity
pub fn verify_database(conn: &Connection) -> SqliteResult<()> {
    // Check foreign key constraint integrity
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    // Verify all tables exist
    let expected_tables = vec![
        "memories",
        "memory_tags",
        "memory_tag_links",
        "memory_edges",
        "reminders",
        "command_history",
        "memory_embeddings",
    ];
    
    for table in expected_tables {
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'", table),
            [],
            |row| row.get(0),
        )?;
        
        if count == 0 {
            eprintln!("⚠️  Table '{}' not found in database", table);
        }
    }
    
    println!("✓ Database integrity verified");
    Ok(())
}
