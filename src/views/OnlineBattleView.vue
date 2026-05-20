<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  useOthello,
  CELL_SIZE,
  PADDING,
  cellBitIndex,
} from "../composables/useOthello";

const router = useRouter();

const {
  black,
  white,
  blackScore,
  whiteScore,
  currentTurn,
  gameOver,
  errorMsg,
  flipAnim,
  canvasRef,
  startGame,
  applyMove,
  drawBoard,
  turnLabel,
  winnerLabel,
} = useOthello();

// ── 页面状态机 ──────────────────────────────────
type PageState = "connecting" | "matching" | "playing" | "finished";
const pageState = ref<PageState>("connecting");
const myColor = ref<"black" | "white" | null>(null);

let unlisten: UnlistenFn | null = null;

// ── 服务器消息处理 ─────────────────────────────ws─
interface ServerMsg {
  type: string;
  pos?: number;
  color?: string;
  message?: string;
}

async function handleServerEvent(event: { payload: string }) {
  let msg: ServerMsg;
  try {
    msg = JSON.parse(event.payload);
  } catch {
    return;
  }

  switch (msg.type) {
    case "match_found":
      myColor.value = (msg.color as "black" | "white") || "black";
      pageState.value = "playing";
      errorMsg.value = "";
      await startGame();
      break;

    case "opponent_move":
      if (msg.pos !== undefined && !gameOver.value && !flipAnim.value) {
        await applyMove(
          black.value,
          white.value,
          msg.pos,
          currentTurn.value === "black",
        );
      }
      break;

    case "opponent_left":
      errorMsg.value = "对手已离开房间";
      pageState.value = "finished";
      break;

    case "opponent_gave_up":
      errorMsg.value = "对手认输，你赢了！";
      pageState.value = "finished";
      break;

    case "disconnected":
      errorMsg.value = "与服务器断开连接";
      pageState.value = "connecting";
      myColor.value = null;
      break;

    case "error":
      errorMsg.value = msg.message || "服务器错误";
      break;
  }
}

// ── 用户操作 ──────────────────────────────────

async function autoConnectAndMatch() {
  errorMsg.value = "";
  pageState.value = "connecting";
  try {
    await invoke("connect_server");
    // 连接成功后自动开始匹配
    pageState.value = "matching";
    if (unlisten) unlisten();
    unlisten = await listen<string>("match_event", handleServerEvent);
    await invoke("find_match");
  } catch (e) {
    errorMsg.value = `连接失败: ${e}`;
    pageState.value = "connecting";
  }
}

async function retryMatching() {
  errorMsg.value = "";
  try {
    await invoke("find_match");
    pageState.value = "matching";
  } catch (e) {
    errorMsg.value = `匹配失败: ${e}`;
  }
}

// ── 棋盘点击（联机版）──────────────────────────
function bb(s: string): bigint {
  try {
    return BigInt(s);
  } catch {
    return 0n;
  }
}

async function onBoardClick(e: MouseEvent) {
  if (pageState.value !== "playing") return;
  if (!myColor.value) return;
  if (currentTurn.value !== myColor.value) return;
  if (gameOver.value || flipAnim.value) return;

  const canvas = canvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const scaleX =
    canvas.width / (rect.width * (window.devicePixelRatio || 1));
  const scaleY =
    canvas.height / (rect.height * (window.devicePixelRatio || 1));
  const x = (e.clientX - rect.left) * scaleX;
  const y = (e.clientY - rect.top) * scaleY;

  const col = Math.floor((x - PADDING) / CELL_SIZE);
  const row = Math.floor((y - PADDING) / CELL_SIZE);
  if (col < 0 || col > 7 || row < 0 || row > 7) return;

  const bitIndex = cellBitIndex(row, col);

  // 粗略检查：该位置已有棋子
  if ((bb(black.value) >> BigInt(bitIndex)) & 1n) return;
  if ((bb(white.value) >> BigInt(bitIndex)) & 1n) return;

  const result = await applyMove(
    black.value,
    white.value,
    bitIndex,
    currentTurn.value === "black",
  );

  if (result) {
    // 本地落子成功 → 发送到服务器
    try {
      await invoke("online_send_move", { posIndex: bitIndex });
    } catch (e) {
      errorMsg.value = `发送落子失败: ${e}`;
    }
  }
}

// ── 认输 ─────────────────────────────────────
async function giveUp() {
  try {
    await invoke("online_give_up");
    errorMsg.value = "你已认输";
    pageState.value = "finished";
  } catch (e) {
    errorMsg.value = `操作失败: ${e}`;
  }
}

function goBack() {
  cleanup();
  router.push("/");
}

function cleanup() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  invoke("disconnect_server").catch(() => {});
}

// ── 生命周期 ──────────────────────────────────
onMounted(async () => {
  await nextTick();
  drawBoard();
  // 自动连接服务器并开始匹配
  await autoConnectAndMatch();
});

onUnmounted(() => {
  cleanup();
});
</script>

<template>
  <div class="online-root">
    <!-- ═══ 顶栏 ═══ -->
    <div class="top-bar">
      <button class="back-btn" @click="goBack">← 返回</button>
      <h1 class="page-title">联网对战</h1>
      <div class="spacer"></div>
    </div>

    <!-- ═══ 状态栏（对局中显示） ═══ -->
    <div
      v-if="pageState === 'playing' || pageState === 'finished'"
      class="info-bar"
    >
      <div class="score black-score">
        <span class="piece-icon">⚫</span>
        <span>黑方 <strong>{{ blackScore }}</strong> 子</span>
      </div>

      <div class="turn-indicator" v-if="!gameOver && pageState === 'playing'">
        <span v-if="myColor">
          轮到：<strong>{{ turnLabel() }}</strong>
          <span class="role-tag">
            {{ currentTurn === myColor ? '(你)' : '(对手)' }}
          </span>
        </span>
        <span class="my-color-tag">
          你执
          <strong>{{ myColor === 'black' ? '⚫ 黑' : '⚪ 白' }}</strong>
        </span>
      </div>
      <div class="turn-indicator winner" v-else-if="gameOver">
        {{ winnerLabel() }}
      </div>
      <div class="turn-indicator" v-else>对局结束</div>

      <div class="score white-score">
        <span class="piece-icon">⚪</span>
        <span>白方 <strong>{{ whiteScore }}</strong> 子</span>
      </div>
    </div>

    <!-- ═══ 棋盘 ═══ -->
    <canvas
      ref="canvasRef"
      class="board-canvas"
      :class="{
        'my-turn':
          pageState === 'playing' && currentTurn === myColor && !gameOver,
      }"
      @click="onBoardClick"
    ></canvas>

    <!-- ═══ 连接中 ═══ -->
    <div v-if="pageState === 'connecting'" class="panel matching-panel">
      <div class="spinner"></div>
      <p class="matching-text">正在连接服务器...</p>
      <button class="action-btn primary" @click="autoConnectAndMatch">
        重新连接
      </button>
    </div>

    <!-- ═══ 匹配面板 ═══ -->
    <div v-if="pageState === 'matching'" class="panel matching-panel">
      <div class="spinner"></div>
      <p class="matching-text">正在寻找对手...</p>
      <div class="match-buttons">
        <button class="action-btn secondary" @click="retryMatching">
          重新匹配
        </button>
      </div>
    </div>

    <!-- ═══ 对局操作按钮 ═══ -->
    <div v-if="pageState === 'playing' && !gameOver" class="bottom-bar">
      <button class="action-btn danger" @click="giveUp">🏳️ 认输</button>
    </div>
    <div v-if="pageState === 'finished'" class="bottom-bar">
      <button
        class="action-btn primary"
        @click="pageState = 'matching'; errorMsg = ''; retryMatching()"
      >
        再来一局
      </button>
    </div>

    <!-- ═══ 错误信息 ═══ -->
    <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
  </div>
</template>

<style scoped>
.online-root {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 16px 32px;
  max-width: 520px;
}

/* ── 顶栏 ── */
.top-bar {
  display: flex;
  align-items: center;
  width: 100%;
  max-width: 480px;
  margin-bottom: 14px;
  gap: 10px;
}

.back-btn {
  padding: 6px 14px;
  font-size: 0.9rem;
  border: 1px solid #444;
  border-radius: 8px;
  background: #2a2a2a;
  color: #ccc;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}
.back-btn:hover {
  background: #3a3a3a;
  border-color: #666;
}

.page-title {
  flex: 1;
  text-align: center;
  font-size: 1.2rem;
  letter-spacing: 1px;
}

.spacer {
  width: 60px;
}

/* ── 信息栏 ── */
.info-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  max-width: 456px;
  padding: 8px 4px;
  gap: 6px;
}

.score {
  font-size: 0.85rem;
  color: #aaa;
  white-space: nowrap;
}
.score strong {
  color: #fff;
}
.piece-icon {
  margin-right: 2px;
}

.turn-indicator {
  text-align: center;
  font-size: 0.9rem;
  color: #ccc;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}
.turn-indicator.winner {
  font-size: 1rem;
  font-weight: 600;
  color: #ffd700;
}
.role-tag {
  font-size: 0.75rem;
  color: #888;
}
.my-color-tag {
  font-size: 0.75rem;
  color: #666;
}

/* ── 棋盘 ── */
.board-canvas {
  border-radius: 12px;
  cursor: default;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  margin: 10px 0;
}
.board-canvas.my-turn {
  cursor: pointer;
}

/* ── 面板 ── */
.panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  margin-top: 30px;
  padding: 28px 32px;
  border-radius: 14px;
  background: #1e1e1e;
  border: 1px solid #333;
  width: 100%;
  max-width: 400px;
}

.input-label {
  font-size: 0.85rem;
  color: #888;
  align-self: flex-start;
}

.input-row {
  display: flex;
  gap: 10px;
  width: 100%;
}

.url-input {
  flex: 1;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid #444;
  background: #2a2a2a;
  color: #ddd;
  font-size: 0.95rem;
  outline: none;
  transition: border-color 0.2s;
}
.url-input:focus {
  border-color: #5a9;
}

.hint {
  font-size: 0.75rem;
  color: #555;
  margin: 0;
}

/* ── 匹配中 ── */
.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid #333;
  border-top-color: #5a9;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.matching-text {
  font-size: 1rem;
  color: #ccc;
}

.match-buttons {
  display: flex;
  gap: 10px;
}

/* ── 按钮 ── */
.action-btn {
  padding: 10px 24px;
  font-size: 0.95rem;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}
.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.primary {
  background: #3a7d4a;
  color: #fff;
}
.primary:hover:not(:disabled) {
  background: #4a9d5e;
}

.secondary {
  background: #444;
  color: #ccc;
}
.secondary:hover:not(:disabled) {
  background: #555;
}

.danger {
  background: #8b3a3a;
  color: #fff;
}
.danger:hover:not(:disabled) {
  background: #a04040;
}

/* ── 底部 ── */
.bottom-bar {
  display: flex;
  justify-content: center;
  gap: 10px;
  margin-top: 14px;
}

.error {
  color: #e05555;
  font-size: 0.85rem;
  margin-top: 8px;
  text-align: center;
}
</style>
