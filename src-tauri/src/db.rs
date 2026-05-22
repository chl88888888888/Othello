//! Database module — Game record storage and queries
//! Uses SQLite (rusqlite bundled) for persistent game data

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// ---------- Data Structures ----------

/// Single move record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub pos_index: u32,
    pub is_black_turn: bool,
}

/// A game record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub id: i64,
    pub black_score: u32,
    pub white_score: u32,
    pub winner: Option<String>,   // "black" | "white" | null(draw)
    pub moves: Vec<MoveRecord>,
    pub created_at: String,
}

/// Game summary (for list view, without move details)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: i64,
    pub black_score: u32,
    pub white_score: u32,
    pub winner: Option<String>,
    pub total_moves: usize,
    pub created_at: String,
}

/// Overall win/loss stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStats {
    pub total_games: u32,
    pub black_wins: u32,
    pub white_wins: u32,
    pub draws: u32,
}

// ---------- Database Manager ----------

#[derive(Debug)]
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database, auto-create tables
    pub fn open(db_path: PathBuf) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create database directory: {e}"))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS games (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                black_score INTEGER NOT NULL,
                white_score INTEGER NOT NULL,
                winner      TEXT,
                moves       TEXT NOT NULL,
                created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );"
        ).map_err(|e| format!("Failed to create table: {e}"))?;

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    /// Save a game record
    pub fn save_game(
        &self,
        black_score: u32,
        white_score: u32,
        winner: Option<String>,
        moves: Vec<MoveRecord>,
    ) -> Result<i64, String> {
        let moves_json = serde_json::to_string(&moves)
            .map_err(|e| format!("Failed to serialize moves: {e}"))?;

        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute(
            "INSERT INTO games (black_score, white_score, winner, moves)
             VALUES (?1, ?2, ?3, ?4)",
            params![black_score, white_score, winner, moves_json],
        ).map_err(|e| format!("Failed to save game: {e}"))?;

        Ok(conn.last_insert_rowid())
    }

    /// Get all game summaries (ordered by time descending)
    pub fn get_game_list(&self) -> Result<Vec<GameSummary>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, black_score, white_score, winner, moves, created_at
                 FROM games ORDER BY id DESC"
            )
            .map_err(|e| format!("Query failed: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                let moves_json: String = row.get(4)?;
                let moves: Vec<MoveRecord> =
                    serde_json::from_str(&moves_json).unwrap_or_default();
                Ok(GameSummary {
                    id: row.get(0)?,
                    black_score: row.get(1)?,
                    white_score: row.get(2)?,
                    winner: row.get(3)?,
                    total_moves: moves.len(),
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to iterate results: {e}"))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| format!("Failed to read row: {e}"))?);
        }
        Ok(list)
    }

    /// Get full game record by ID (includes moves for replay)
    pub fn get_game(&self, id: i64) -> Result<GameRecord, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.query_row(
            "SELECT id, black_score, white_score, winner, moves, created_at
             FROM games WHERE id = ?1",
            params![id],
            |row| {
                let moves_json: String = row.get(4)?;
                let moves: Vec<MoveRecord> =
                    serde_json::from_str(&moves_json).unwrap_or_default();
                Ok(GameRecord {
                    id: row.get(0)?,
                    black_score: row.get(1)?,
                    white_score: row.get(2)?,
                    winner: row.get(3)?,
                    moves,
                    created_at: row.get(5)?,
                })
            },
        ).map_err(|e| format!("Game #{id} not found: {e}"))
    }

    /// Get overall win/loss stats
    pub fn get_stats(&self) -> Result<GameStats, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;

        let total: u32 = conn
            .query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))
            .map_err(|e| format!("Stats query failed: {e}"))?;

        let black_wins: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE winner = 'black'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Stats query failed: {e}"))?;

        let white_wins: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE winner = 'white'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Stats query failed: {e}"))?;

        let draws: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE winner IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Stats query failed: {e}"))?;

        Ok(GameStats {
            total_games: total,
            black_wins,
            white_wins,
            draws,
        })
    }

    /// Delete a specific game
    pub fn delete_game(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock failed: {e}"))?;
        conn.execute("DELETE FROM games WHERE id = ?1", params![id])
            .map_err(|e| format!("Delete failed: {e}"))?;
        Ok(())
    }
}
