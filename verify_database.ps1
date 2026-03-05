# Database Verification Script for Noddy (Windows PowerShell)
# This script verifies that the SQLite database schema is properly initialized

$ErrorActionPreference = "SilentlyContinue"

# Windows database path
$DB_PATH = "$env:APPDATA\noddy\noddy.db"

# Color output
function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Error2 {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

function Write-Warning2 {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Write-Info {
    param([string]$Message)
    Write-Host "$Message" -ForegroundColor Cyan
}

# Check if sqlite3 is available
$sqlite3_check = Get-Command sqlite3 -ErrorAction SilentlyContinue
if (-not $sqlite3_check) {
    Write-Error2 "sqlite3 not found in PATH"
    Write-Warning2 "Please install sqlite3 or use: choco install sqlite"
    exit 1
}

Write-Info "=========================================="
Write-Info "Noddy Database Verification"
Write-Info "=========================================="
Write-Host ""

# Check if database file exists
if (-not (Test-Path $DB_PATH)) {
    Write-Warning2 "Database file not found at: $DB_PATH"
    Write-Info "Start Noddy application to initialize the database."
    exit 1
}

Write-Success "Database file found: $DB_PATH"
Write-Host ""

# Function to check if table exists
function Check-Table {
    param([string]$TableName)
    try {
        $count = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$TableName';"
        if ($count -eq 1) {
            Write-Success "Table: $TableName"
            return $true
        } else {
            Write-Error2 "Table: $TableName (MISSING)"
            return $false
        }
    } catch {
        Write-Error2 "Table: $TableName (ERROR: $_)"
        return $false
    }
}

# Function to count records in table
function Count-Records {
    param([string]$TableName)
    try {
        $count = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM $TableName;" 2>$null
        Write-Host "  └─ Records: $count" -ForegroundColor Gray
    } catch {
        Write-Host "  └─ Records: ERROR" -ForegroundColor Red
    }
}

# Check all required tables
Write-Info "Checking Tables:"
Write-Info "================"

$all_tables_exist = $true

$tables = @(
    "memories",
    "memory_tags",
    "memory_tag_links",
    "memory_edges",
    "reminders",
    "command_history",
    "memory_embeddings"
)

foreach ($table in $tables) {
    if (Check-Table $table) {
        Count-Records $table
    } else {
        $all_tables_exist = $false
    }
}

Write-Host ""

# Check indexes
Write-Info "Checking Indexes:"
Write-Info "=================="

function Check-Index {
    param([string]$IndexName)
    try {
        $count = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='$IndexName';"
        if ($count -eq 1) {
            Write-Success "Index: $IndexName"
        } else {
            Write-Warning2 "Index: $IndexName (MISSING)"
        }
    } catch {
        Write-Error2 "Index: $IndexName (ERROR)"
    }
}

$indexes = @(
    "idx_memories_created_at",
    "idx_memories_updated_at",
    "idx_memories_importance",
    "idx_memories_source",
    "idx_memory_tags_tag",
    "idx_memory_tag_links_tag_id",
    "idx_memory_tag_links_memory_id",
    "idx_memory_edges_source",
    "idx_memory_edges_target",
    "idx_memory_edges_relationship",
    "idx_memory_edges_created_at",
    "idx_reminders_trigger_at",
    "idx_reminders_status",
    "idx_reminders_memory_id",
    "idx_command_history_timestamp",
    "idx_command_history_intent",
    "idx_command_history_status"
)

foreach ($index in $indexes) {
    Check-Index $index
}

Write-Host ""

# Check foreign key constraint
Write-Info "Checking Configuration:"
Write-Info "======================="

try {
    $fk_pragma = & sqlite3 $DB_PATH "PRAGMA foreign_keys;"
    if ($fk_pragma -eq 1) {
        Write-Success "Foreign Key Constraints: Enabled"
    } else {
        Write-Warning2 "Foreign Key Constraints: Disabled (will be enabled on connection)"
    }
} catch {
    Write-Warning2 "Foreign Key Constraints: Unable to verify"
}

Write-Host ""

# Verify schema
Write-Info "Verifying Schema Integrity:"
Write-Info "==========================="

try {
    $integrity = & sqlite3 $DB_PATH "PRAGMA integrity_check;"
    if ($integrity -eq "ok") {
        Write-Success "Database integrity: OK"
    } else {
        Write-Error2 "Database integrity: ISSUES DETECTED"
        Write-Host $integrity -ForegroundColor Red
    }
} catch {
    Write-Error2 "Database integrity: ERROR ($_)"
}

Write-Host ""

# Summary
if ($all_tables_exist) {
    Write-Success "All systems operational!"
    Write-Host ""
    
    Write-Info "Database Statistics:"
    try {
        $total_memories = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM memories;"
        $total_reminders = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM reminders;"
        $total_commands = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM command_history;"
        
        Write-Host "  • Memories: $total_memories"
        Write-Host "  • Reminders: $total_reminders"
        Write-Host "  • Commands logged: $total_commands"
    } catch {
        Write-Warning2 "Unable to retrieve statistics"
    }
} else {
    Write-Error2 "Database schema incomplete"
    Write-Info "Restart Noddy to initialize or recreate the database."
    exit 1
}

# Show sample queries
Write-Host ""
Write-Info "Useful Commands:"
Write-Info "================"
Write-Host ""
Write-Host '# View all tables' -ForegroundColor Gray
Write-Host "sqlite3 '$DB_PATH' '.tables'" -ForegroundColor DarkGray
Write-Host ""
Write-Host '# View memories table schema' -ForegroundColor Gray
Write-Host "sqlite3 '$DB_PATH' '.schema memories'" -ForegroundColor DarkGray
Write-Host ""
Write-Host '# Query recent memories' -ForegroundColor Gray
Write-Host "sqlite3 '$DB_PATH' 'SELECT id, content, created_at FROM memories ORDER BY created_at DESC LIMIT 5;'" -ForegroundColor DarkGray
Write-Host ""
Write-Host '# Count records by type' -ForegroundColor Gray
Write-Host 'sqlite3 "$DB_PATH" "SELECT ''memories'' as type, COUNT(*) as count FROM memories UNION SELECT ''tags'', COUNT(*) FROM memory_tags;"' -ForegroundColor DarkGray
Write-Host ""
Write-Host '# Backup database' -ForegroundColor Gray
Write-Host "Copy-Item '$DB_PATH' '$DB_PATH.backup'" -ForegroundColor DarkGray
Write-Host ""

Write-Host "✓ Verification complete!" -ForegroundColor Green
