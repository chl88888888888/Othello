//! 数据库模块 — 对局记录存储与查询
//! 使用 SQLite (rusqlite bundled) 持久化对局数据

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// ---------- 数据结构 ----------

/// 单步落子记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub pos_index: u32,
    pub is_black_turn: bool,
}

/// 数据库中的一条对局记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub id: i64,
    pub black_score: u32,
    pub white_score: u32,
    pub winner: Option<String>,   // "black" | "white" | null(平局)
    pub moves: Vec<MoveRecord>,
    pub created_at: String,
}

/// 对局摘要（列表用，不含走法详情）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: i64,
    pub black_score: u32,
    pub white_score: u32,
    pub winner: Option<String>,
    pub total_moves: usize,
    pub created_at: String,
}

/// 总胜负统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStats {
    pub total_games: u32,
    pub black_wins: u32,
    pub white_wins: u32,
    pub draws: u32,
}

// ---------- 数据库管理器 ----------

#[derive(Debug)]
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 打开（或创建）数据库，自动建表
    pub fn open(db_path: PathBuf) -> Result<Self, String> {
        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建数据库目录: {e}"))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("无法打开数据库: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS games (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                black_score INTEGER NOT NULL,
                white_score INTEGER NOT NULL,
                winner      TEXT,
                moves       TEXT NOT NULL,
                created_at  TEXT NOT NULL DEFAULT (datetime('now','localtime'))
            );"
        ).map_err(|e| format!("建表失败: {e}"))?;

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    /// 保存一局对局
    pub fn save_game(
        &self,
        black_score: u32,
        white_score: u32,
        winner: Option<String>,
        moves: Vec<MoveRecord>,
    ) -> Result<i64, String> {
        let moves_json = serde_json::to_string(&moves)
            .map_err(|e| format!("序列化走法失败: {e}"))?;

        let conn = self.conn.lock().map_err(|e| format!("锁失败: {e}"))?;
        conn.execute(
            "INSERT INTO games (black_score, white_score, winner, moves)
             VALUES (?1, ?2, ?3, ?4)",
            params![black_score, white_score, winner, moves_json],
        ).map_err(|e| format!("保存对局失败: {e}"))?;

        Ok(conn.last_insert_rowid())
    }

    /// 获取所有对局摘要列表（按时间倒序）
    pub fn get_game_list(&self) -> Result<Vec<GameSummary>, String> {
        let conn = self.conn.lock().map_err(|e| format!("锁失败: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, black_score, white_score, winner, moves, created_at
                 FROM games ORDER BY id DESC"
            )
            .map_err(|e| format!("查询失败: {e}"))?;

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
            .map_err(|e| format!("遍历结果失败: {e}"))?;

        let mut list = Vec::new();
        for row in rows {
            list.push(row.map_err(|e| format!("读取行失败: {e}"))?);
        }
        Ok(list)
    }

    /// 根据 ID 获取单局完整记录（含走法，用于回放）
    pub fn get_game(&self, id: i64) -> Result<GameRecord, String> {
        let conn = self.conn.lock().map_err(|e| format!("锁失败: {e}"))?;
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
        ).map_err(|e| format!("未找到对局 #{id}: {e}"))
    }

    /// 获取总胜负统计
    pub fn get_stats(&self) -> Result<GameStats, String> {
        let conn = self.conn.lock().map_err(|e| format!("锁失败: {e}"))?;

        let total: u32 = conn
            .query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))
            .map_err(|e| format!("统计失败: {e}"))?;

        let black_wins: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE winner = 'black'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("统计失败: {e}"))?;

        let white_wins: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE winner = 'white'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("统计失败: {e}"))?;

        let draws: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE winner IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("统计失败: {e}"))?;

        Ok(GameStats {
            total_games: total,
            black_wins,
            white_wins,
            draws,
        })
    }

    /// 删除指定对局
    pub fn delete_game(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("锁失败: {e}"))?;
        conn.execute("DELETE FROM games WHERE id = ?1", params![id])
            .map_err(|e| format!("删除失败: {e}"))?;
        Ok(())
    }
}
