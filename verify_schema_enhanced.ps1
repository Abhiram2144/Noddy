# Enhanced Database Schema Verification Script for Noddy
# This script verifies that the SQLite database schema includes user_id columns

$ErrorActionPreference = "SilentlyContinue"

# Windows database path
$DB_PATH = "$env:APPDATA\noddy\noddy.db"

# Color output functions
function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Fail {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

function Write-Warn {
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
    Write-Fail "sqlite3 not found in PATH"
    Write-Warn "Please install sqlite3 or use: choco install sqlite"
    exit 1
}

Write-Info "=========================================="
Write-Info "Noddy Enhanced Schema Verification"
Write-Info "=========================================="
Write-Host ""

# Check if database file exists
if (-not (Test-Path $DB_PATH)) {
    Write-Warn "Database file not found at: $DB_PATH"
    Write-Info "Start Noddy application to initialize the database."
    exit 1
}

Write-Success "Database file found: $DB_PATH"
Write-Host ""

# Function to check if column exists in table
function Check-Column {
    param(
        [string]$TableName,
        [string]$ColumnName
    )
    try {
        $result = & sqlite3 $DB_PATH "PRAGMA table_info($TableName);"
        if ($result -match $ColumnName) {
            Write-Success "  └─ Column '$ColumnName' exists"
            return $true
        } else {
            Write-Fail "  └─ Column '$ColumnName' MISSING"
            return $false
        }
    } catch {
        Write-Fail "  └─ Error checking column '$ColumnName': $_"
        return $false
    }
}

# Function to display table schema
function Show-TableSchema {
    param([string]$TableName)
    try {
        Write-Host "  Schema:" -ForegroundColor Gray
        $schema = & sqlite3 $DB_PATH "PRAGMA table_info($TableName);"
        foreach ($line in $schema) {
            Write-Host "    $line" -ForegroundColor DarkGray
        }
    } catch {
        Write-Host "  Error getting schema: $_" -ForegroundColor Red
    }
}

# Check critical tables for user_id column
Write-Info "Checking Multi-User Support (user_id columns):"
Write-Info "=============================================="

$critical_checks = @(
    @{Table="memories"; RequiredColumns=@("id", "user_id", "content", "created_at")},
    @{Table="memory_edges"; RequiredColumns=@("id", "user_id", "source_memory_id", "target_memory_id")},
    @{Table="reminders"; RequiredColumns=@("id", "user_id", "content", "trigger_at")},
    @{Table="command_history"; RequiredColumns=@("id", "user_id", "command_text", "timestamp")},
    @{Table="users"; RequiredColumns=@("id", "email", "password_hash", "created_at")},
    @{Table="sessions"; RequiredColumns=@("id", "user_id", "refresh_token", "expires_at")}
)

$all_checks_passed = $true

foreach ($check in $critical_checks) {
    $tableName = $check.Table
    Write-Host ""
    Write-Info "Table: $tableName"
    
    # Check if table exists
    $table_count = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$tableName';"
    if ($table_count -eq 1) {
        Write-Success "  Table exists"
        
        # Check for required columns
        foreach ($col in $check.RequiredColumns) {
            if (-not (Check-Column -TableName $tableName -ColumnName $col)) {
                $all_checks_passed = $false
            }
        }
        
        # Show record count
        try {
            $count = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM $tableName;" 2>$null
            Write-Host "  └─ Records: $count" -ForegroundColor Gray
            
            # For tables with user_id, check for NULL values
            if ($check.RequiredColumns -contains "user_id" -and $tableName -ne "users" -and $tableName -ne "sessions") {
                $null_count = & sqlite3 $DB_PATH "SELECT COUNT(*) FROM $tableName WHERE user_id IS NULL;" 2>$null
                if ($null_count -gt 0) {
                    Write-Warn "  └─ Warning: $null_count records with NULL user_id (legacy data)"
                }
            }
        } catch {
            Write-Host "  └─ Records: ERROR" -ForegroundColor Red
        }
    } else {
        Write-Fail "  Table MISSING"
        $all_checks_passed = $false
    }
}

Write-Host ""
Write-Info "=========================================="

# Summary
if ($all_checks_passed) {
    Write-Success "All schema checks passed!"
    Write-Host ""
    Write-Info "The database schema is correctly configured for multi-user support."
    Write-Info "All critical tables have the required user_id columns."
} else {
    Write-Fail "Some schema checks FAILED!"
    Write-Host ""
    Write-Warn "Action Required:"
    Write-Host "1. Stop the Noddy application if running"
    Write-Host "2. Delete the database file at: $DB_PATH"
    Write-Host "3. Restart Noddy to recreate the database with correct schema"
    Write-Host ""
    Write-Warn "Note: This will delete all existing data. Backup if needed."
}

Write-Host ""

# Display useful queries
Write-Info "Useful Debug Queries:"
Write-Info "===================="
Write-Host "To inspect schema: sqlite3 '$DB_PATH' '.schema memories'"
Write-Host "To check data: sqlite3 '$DB_PATH' 'SELECT * FROM memories LIMIT 5;'"
Write-Host "To check users: sqlite3 '$DB_PATH' 'SELECT id, email FROM users;'"
Write-Host ""
