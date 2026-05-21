<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useOthello, type MoveRecord } from "../composables/useOthello";

const router = useRouter();

// ── 数据库类型 ──
interface GameSummary {
  id: number;
  black_score: number;
  white_score: number;
  winner: string | null;
  total_moves: number;
  created_at: string;
}

interface GameRecord {
  id: number;
  black_score: number;
  white_score: number;
  winner: string | null;
  moves: MoveRecord[];
  created_at: string;
}

interface GameStats {
  total_games: number;
  black_wins: number;
  white_wins: number;
  draws: number;
}

// ── 回放 composable ──
const {
  blackScore,
  whiteScore,
  gameOver,
  errorMsg,
  canvasRef,
  turnLabel,
  winnerLabel,
  isReplaying,
  replayingGameId,
  replayGame,
  stopReplay,
  drawBoard,
} = useOthello();

// ── 响应式数据 ──
const gameList = ref<GameSummary[]>([]);
const stats = ref<GameStats>({ total_games: 0, black_wins: 0, white_wins: 0, draws: 0 });
const loading = ref(true);
const replayError = ref("");

// ── 加载数据 ──
async function loadData() {
  loading.value = true;
  try {
    const [list, s] = await Promise.all([
      invoke<GameSummary[]>("get_game_list"),
      invoke<GameStats>("get_stats"),
    ]);
    gameList.value = list;
    stats.value = s;
  } catch (e) {
    console.error("加载历史数据失败:", e);
  } finally {
    loading.value = false;
  }
}

// ── 回放对局 ──
async function playGame(summary: GameSummary) {
  replayError.value = "";
  try {
    const detail = await invoke<GameRecord>("get_game_detail", { id: summary.id });
    await replayGame(detail.moves, detail.id);
  } catch (e) {
    replayError.value = `加载对局失败: ${e}`;
  }
}

// ── 删除对局 ──
async function deleteGame(id: number) {
  if (!confirm(`确定要删除对局 #${id} 吗？`)) return;
  try {
    await invoke("delete_game", { id });
    await loadData();
  } catch (e) {
    console.error("删除失败:", e);
  }
}

// ── 停止回放 ──
function handleStopReplay() {
  stopReplay();
}

function goBack() {
  stopReplay();
  router.push("/");
}

// ── 辅助 ──
function winnerText(w: string | null): string {
  if (w === "black") return "黑胜";
  if (w === "white") return "白胜";
  return "平局";
}

function winnerClass(w: string | null): string {
  if (w === "black") return "win-black";
  if (w === "white") return "win-white";
  return "win-draw";
}

function formatTime(ts: string): string {
  // created_at 格式: "YYYY-MM-DD HH:MM:SS"
  return ts.replace("T", " ").substring(0, 19);
}

onMounted(async () => {
  await loadData();
  drawBoard();
});
</script>

<template>
  <div class="history-root">
    <!-- 顶栏 -->
    <div class="top-bar">
      <button class="back-btn" @click="goBack">← 返回</button>
      <h1 class="page-title">历史查询</h1>
      <div class="spacer"></div>
    </div>

    <!-- ── 总胜负统计 ── -->
    <div class="stats-panel">
      <div class="stat-item">
        <span class="stat-value">{{ stats.total_games }}</span>
        <span class="stat-label">总对局</span>
      </div>
      <div class="stat-item win-black">
        <span class="stat-value">{{ stats.black_wins }}</span>
        <span class="stat-label">⚫ 黑胜</span>
      </div>
      <div class="stat-item win-white">
        <span class="stat-value">{{ stats.white_wins }}</span>
        <span class="stat-label">⚪ 白胜</span>
      </div>
      <div class="stat-item win-draw">
        <span class="stat-value">{{ stats.draws }}</span>
        <span class="stat-label">平局</span>
      </div>
    </div>

    <!-- ── 回放区域 ── -->
    <div v-show="isReplaying" class="replay-section">
      <div class="replay-header">
        <span class="replay-badge">▶ 正在回放 #{{ replayingGameId }}</span>
        <button class="stop-btn" @click="handleStopReplay">⏹ 停止</button>
      </div>
      <div class="replay-info">
        <span class="score black-score">⚫ {{ blackScore }}</span>
        <span class="turn-text">{{ gameOver ? winnerLabel() : turnLabel() }}</span>
        <span class="score white-score">⚪ {{ whiteScore }}</span>
      </div>
      <canvas ref="canvasRef" class="board-canvas"></canvas>
    </div>

    <!-- ── 对局列表 ── -->
    <div class="list-section">
      <div class="list-header">
        <h2>对局记录</h2>
        <button class="refresh-btn" @click="loadData" :disabled="loading">
          {{ loading ? "加载中..." : "🔄 刷新" }}
        </button>
      </div>

      <p v-if="replayError" class="error-msg">{{ replayError }}</p>
      <p v-if="errorMsg" class="error-msg">{{ errorMsg }}</p>

      <!-- 空状态 -->
      <div v-if="!loading && gameList.length === 0" class="empty-state">
        <span class="empty-icon">📭</span>
        <p>暂无对局记录</p>
        <p class="empty-hint">完成一局人机对战后会自动保存</p>
      </div>

      <!-- 列表 -->
      <div v-else class="game-list">
        <div
          v-for="g in gameList"
          :key="g.id"
          class="game-card"
          :class="{
            'is-replaying': isReplaying && replayingGameId === g.id,
          }"
        >
          <div class="game-main" @click="playGame(g)">
            <div class="game-id">#{{ g.id }}</div>
            <div class="game-result">
              <span :class="'result-tag ' + winnerClass(g.winner)">
                {{ winnerText(g.winner) }}
              </span>
            </div>
            <div class="game-score">
              ⚫ {{ g.black_score }} : {{ g.white_score }} ⚪
            </div>
            <div class="game-meta">
              <span>{{ g.total_moves }} 手</span>
              <span>{{ formatTime(g.created_at) }}</span>
            </div>
          </div>
          <button
            class="delete-btn"
            @click.stop="deleteGame(g.id)"
            title="删除此对局"
          >
            🗑
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.history-root {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 12px 20px;
  width: 100%;
  max-width: 100vw;
  overflow-x: hidden;
}

/* ── 顶栏 ── */
.top-bar {
  display: flex;
  align-items: center;
  width: 100%;
  max-width: 100%;
  margin-bottom: 10px;
  gap: 8px;
}

.back-btn {
  padding: 5px 10px;
  font-size: clamp(0.75rem, 2.5vw, 0.9rem);
  border: 1px solid #444;
  border-radius: 8px;
  background: #2a2a2a;
  color: #ccc;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
  flex-shrink: 0;
}

.back-btn:hover {
  background: #3a3a3a;
  border-color: #666;
}

.page-title {
  flex: 1;
  text-align: center;
  font-size: clamp(1rem, 3.5vw, 1.2rem);
  letter-spacing: 1px;
  white-space: nowrap;
}

.spacer {
  width: 48px;
  flex-shrink: 0;
}

/* ── 统计面板 ── */
.stats-panel {
  display: flex;
  gap: 6px;
  width: 100%;
  max-width: 100%;
  margin-bottom: 12px;
}

.stat-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 4px;
  border-radius: 10px;
  background: #262626;
  border: 1px solid #3a3a3a;
}

.stat-value {
  font-size: clamp(1.1rem, 4vw, 1.5rem);
  font-weight: 700;
  color: #e0e0e0;
}

.stat-label {
  font-size: clamp(0.6rem, 2vw, 0.75rem);
  color: #999;
  margin-top: 2px;
}

.stat-item.win-black .stat-value {
  color: #888;
}

.stat-item.win-white .stat-value {
  color: #ddd;
}

.stat-item.win-draw .stat-value {
  color: #aaa;
}

/* ── 回放区域 ── */
.replay-section {
  width: 100%;
  max-width: 100%;
  margin-bottom: 12px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.replay-header {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  justify-content: center;
}

.replay-badge {
  font-size: clamp(0.8rem, 2.6vw, 0.95rem);
  color: #5a8a5a;
  font-weight: 600;
}

.stop-btn {
  padding: 4px 12px;
  font-size: clamp(0.7rem, 2.2vw, 0.85rem);
  border: 1px solid #944;
  border-radius: 6px;
  background: #3a2020;
  color: #f87171;
  cursor: pointer;
  transition: all 0.2s;
}

.stop-btn:hover {
  background: #502828;
}

.replay-info {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: clamp(0.78rem, 2.5vw, 0.95rem);
}

.turn-text {
  color: #ccc;
  font-weight: 600;
}

.score {
  padding: 3px 10px;
  border-radius: 6px;
  background: #2a2a2a;
  font-size: clamp(0.7rem, 2.3vw, 0.9rem);
}

.board-canvas {
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  max-width: 100%;
  height: auto;
}

/* ── 列表区域 ── */
.list-section {
  width: 100%;
  max-width: 100%;
}

.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.list-header h2 {
  font-size: clamp(0.9rem, 3vw, 1.1rem);
  color: #ccc;
  margin: 0;
}

.refresh-btn {
  padding: 4px 10px;
  font-size: clamp(0.68rem, 2vw, 0.8rem);
  border: 1px solid #444;
  border-radius: 6px;
  background: #2a2a2a;
  color: #aaa;
  cursor: pointer;
  transition: all 0.2s;
}

.refresh-btn:hover:not(:disabled) {
  background: #3a3a3a;
  border-color: #666;
}

.error-msg {
  color: #f87171;
  font-size: clamp(0.7rem, 2.2vw, 0.85rem);
  text-align: center;
  margin: 6px 0;
}

/* ── 空状态 ── */
.empty-state {
  text-align: center;
  padding: 30px 16px;
  color: #777;
}

.empty-icon {
  font-size: 2.4rem;
  display: block;
  margin-bottom: 8px;
}

.empty-hint {
  font-size: clamp(0.7rem, 2vw, 0.82rem);
  color: #555;
  margin-top: 4px;
}

/* ── 对局卡片 ── */
.game-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.game-card {
  display: flex;
  align-items: center;
  border: 1px solid #3a3a3a;
  border-radius: 10px;
  background: #262626;
  transition: all 0.2s;
}

.game-card:hover {
  border-color: #5a5a5a;
}

.game-card.is-replaying {
  border-color: #5a8a5a;
  background: #283028;
}

.game-main {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 10px;
  cursor: pointer;
  min-width: 0;
  flex-wrap: wrap;
}

.game-id {
  font-size: clamp(0.7rem, 2.2vw, 0.85rem);
  color: #777;
  font-weight: 600;
  min-width: 28px;
}

.game-result {
  min-width: 40px;
}

.result-tag {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: clamp(0.65rem, 2vw, 0.78rem);
  font-weight: 600;
}

.result-tag.win-black {
  background: #2a2a2a;
  color: #aaa;
  border: 1px solid #555;
}

.result-tag.win-white {
  background: #333;
  color: #ddd;
  border: 1px solid #666;
}

.result-tag.win-draw {
  background: #2a2a2a;
  color: #999;
  border: 1px solid #444;
}

.game-score {
  flex: 1;
  font-size: clamp(0.75rem, 2.3vw, 0.9rem);
  color: #ccc;
  text-align: center;
  min-width: 60px;
}

.game-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  font-size: clamp(0.6rem, 1.8vw, 0.72rem);
  color: #666;
  gap: 1px;
  flex-shrink: 0;
}

.delete-btn {
  padding: 6px 8px;
  font-size: clamp(0.7rem, 2.1vw, 0.85rem);
  background: none;
  border: none;
  color: #555;
  cursor: pointer;
  transition: color 0.2s;
  border-radius: 0 10px 10px 0;
  flex-shrink: 0;
}

.delete-btn:hover {
  color: #f87171;
  background: rgba(255, 0, 0, 0.08);
}
</style>
