#!/bin/bash
# Database Verification Script for Noddy
# This script verifies that the SQLite database schema is properly initialized

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Platform-specific database path
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    DB_PATH="${HOME}/.local/share/noddy/noddy.db"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    DB_PATH="${HOME}/Library/Application Support/noddy/noddy.db"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    DB_PATH="${APPDATA}/noddy/noddy.db"
else
    echo -e "${RED}Unknown OS. Please check database path manually.${NC}"
    exit 1
fi

echo "=========================================="
echo "Noddy Database Verification"
echo "=========================================="
echo ""

# Check if database file exists
if [ ! -f "$DB_PATH" ]; then
    echo -e "${YELLOW}⚠️  Database file not found at: $DB_PATH${NC}"
    echo "Start Noddy application to initialize the database."
    exit 1
fi

echo -e "${GREEN}✓${NC} Database file found: $DB_PATH"
echo ""

# Function to check if table exists
check_table() {
    local table=$1
    local count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$table';" 2>/dev/null)
    if [ "$count" -eq 1 ]; then
        echo -e "${GREEN}✓${NC} Table: $table"
        return 0
    else
        echo -e "${RED}✗${NC} Table: $table (MISSING)"
        return 1
    fi
}

# Function to count records in table
count_records() {
    local table=$1
    local count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM $table;" 2>/dev/null)
    echo -e "  └─ Records: $count"
}

# Check all required tables
echo "Checking Tables:"
echo "================"

all_tables_exist=true

for table in memories memory_tags memory_tag_links memory_edges reminders command_history memory_embeddings; do
    if check_table "$table"; then
        count_records "$table"
    else
        all_tables_exist=false
    fi
done

echo ""

# Check indexes
echo "Checking Indexes:"
echo "=================="

check_index() {
    local index=$1
    local count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='$index';" 2>/dev/null)
    if [ "$count" -eq 1 ]; then
        echo -e "${GREEN}✓${NC} Index: $index"
    else
        echo -e "${YELLOW}⚠${NC} Index: $index (MISSING)"
    fi
}

indexes=(
    "idx_memories_created_at"
    "idx_memories_updated_at"
    "idx_memories_importance"
    "idx_memories_source"
    "idx_memory_tags_tag"
    "idx_memory_tag_links_tag_id"
    "idx_memory_tag_links_memory_id"
    "idx_memory_edges_source"
    "idx_memory_edges_target"
    "idx_memory_edges_relationship"
    "idx_memory_edges_created_at"
    "idx_reminders_trigger_at"
    "idx_reminders_status"
    "idx_reminders_memory_id"
    "idx_command_history_timestamp"
    "idx_command_history_intent"
    "idx_command_history_status"
)

for index in "${indexes[@]}"; do
    check_index "$index"
done

echo ""

# Check foreign key constraint
echo "Checking Configuration:"
echo "======================="

fk_pragma=$(sqlite3 "$DB_PATH" "PRAGMA foreign_keys;" 2>/dev/null)
if [ "$fk_pragma" -eq 1 ]; then
    echo -e "${GREEN}✓${NC} Foreign Key Constraints: Enabled"
else
    echo -e "${YELLOW}⚠${NC} Foreign Key Constraints: Disabled (will be enabled on connection)"
fi

echo ""

# Verify schema
echo "Verifying Schema Integrity:"
echo "==========================="

integrity=$(sqlite3 "$DB_PATH" "PRAGMA integrity_check;" 2>/dev/null)
if [ "$integrity" == "ok" ]; then
    echo -e "${GREEN}✓${NC} Database integrity: OK"
else
    echo -e "${RED}✗${NC} Database integrity: ISSUES DETECTED"
    echo "$integrity"
fi

echo ""

# Summary
if [ "$all_tables_exist" = true ]; then
    echo -e "${GREEN}✓ All systems operational!${NC}"
    echo ""
    echo "Database Statistics:"
    total_memories=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM memories;" 2>/dev/null)
    total_reminders=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM reminders;" 2>/dev/null)
    total_commands=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM command_history;" 2>/dev/null)
    
    echo "  • Memories: $total_memories"
    echo "  • Reminders: $total_reminders"
    echo "  • Commands logged: $total_commands"
else
    echo -e "${RED}✗ Database schema incomplete${NC}"
    echo "Restart Noddy to initialize or recreate the database."
    exit 1
fi

# Show sample queries
echo ""
echo "Useful Commands:"
echo "================"
echo ""
echo "# View all tables"
echo "sqlite3 \"$DB_PATH\" \".tables\""
echo ""
echo "# View memories table schema"
echo "sqlite3 \"$DB_PATH\" \".schema memories\""
echo ""
echo "# Query recent memories"
echo "sqlite3 \"$DB_PATH\" \"SELECT id, content, created_at FROM memories ORDER BY created_at DESC LIMIT 5;\""
echo ""
echo "# Count records by type"
echo "sqlite3 \"$DB_PATH\" \"SELECT 'memories' as type, COUNT(*) as count FROM memories UNION SELECT 'tags', COUNT(*) FROM memory_tags;\""
echo ""
echo "# Backup database"
echo "cp \"$DB_PATH\" \"$DB_PATH.backup\""
echo ""
