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

// ── Page state machine ────────────────────────────
type PageState = "connecting" | "matching" | "playing" | "finished";
const pageState = ref<PageState>("connecting");
const myColor = ref<"black" | "white" | null>(null);

let unlisten: UnlistenFn | null = null;

// ── Server message handling ───────────────────────
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

// ── User actions ─────────────────────────────────

async function autoConnectAndMatch() {
  errorMsg.value = "";
  pageState.value = "connecting";
  try {
    await invoke("connect_server");
    // Auto-start matching after connection
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

// ── Board click (online version) ──────────────────
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

  // Rough check: position already occupied
  if ((bb(black.value) >> BigInt(bitIndex)) & 1n) return;
  if ((bb(white.value) >> BigInt(bitIndex)) & 1n) return;

  const result = await applyMove(
    black.value,
    white.value,
    bitIndex,
    currentTurn.value === "black",
  );

  if (result) {
    // Local move succeeded → send to server
    try {
      await invoke("online_send_move", { posIndex: bitIndex });
    } catch (e) {
      errorMsg.value = `发送落子失败: ${e}`;
    }
  }
}

// ── Give up ──────────────────────────────────────
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

// ── Lifecycle ────────────────────────────────────
onMounted(async () => {
  await nextTick();
  drawBoard();
  // Auto-connect and start matching
  await autoConnectAndMatch();
});

onUnmounted(() => {
  cleanup();
});
</script>

<template>
  <div class="online-root">
    <!-- Top bar -->
    <div class="top-bar">
      <button class="back-btn" @click="goBack">← 返回</button>
      <h1 class="page-title">联网对战</h1>
      <div class="spacer"></div>
    </div>

    <!-- Info bar (shown during gameplay) -->
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

    <!-- Board -->
    <canvas
      ref="canvasRef"
      class="board-canvas"
      :class="{
        'my-turn':
          pageState === 'playing' && currentTurn === myColor && !gameOver,
      }"
      @click="onBoardClick"
    ></canvas>

    <!-- Connecting -->
    <div v-if="pageState === 'connecting'" class="panel matching-panel">
      <div class="spinner"></div>
      <p class="matching-text">正在连接服务器...</p>
      <button class="action-btn primary" @click="autoConnectAndMatch">
        重新连接
      </button>
    </div>

    <!-- Matching panel -->
    <div v-if="pageState === 'matching'" class="panel matching-panel">
      <div class="spinner"></div>
      <p class="matching-text">正在寻找对手...</p>
      <div class="match-buttons">
        <button class="action-btn secondary" @click="retryMatching">
          重新匹配
        </button>
      </div>
    </div>

    <!-- Game action buttons -->
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

    <!-- Error message -->
    <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
  </div>
</template>

<style scoped>
.online-root {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 12px 20px;
  width: 100%;
  max-width: 100vw;
  overflow-x: hidden;
}

/* ── Top bar ── */
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

/* ── Info bar ── */
.info-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  max-width: 100%;
  padding: 6px 2px;
  gap: 4px;
  flex-wrap: wrap;
}

.score {
  font-size: clamp(0.7rem, 2.2vw, 0.85rem);
  color: #aaa;
  white-space: nowrap;
}
.score strong {
  color: #fff;
}
.piece-icon {
  margin-right: 1px;
}

.turn-indicator {
  text-align: center;
  font-size: clamp(0.75rem, 2.4vw, 0.9rem);
  color: #ccc;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1px;
}
.turn-indicator.winner {
  font-size: clamp(0.85rem, 2.8vw, 1rem);
  font-weight: 600;
  color: #ffd700;
}
.role-tag {
  font-size: clamp(0.62rem, 1.9vw, 0.75rem);
  color: #888;
}
.my-color-tag {
  font-size: clamp(0.62rem, 1.9vw, 0.75rem);
  color: #666;
}

/* ── Board ── */
.board-canvas {
  border-radius: 12px;
  cursor: default;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  margin: 8px 0;
  max-width: 100%;
  height: auto;
}
.board-canvas.my-turn {
  cursor: pointer;
}

/* ── Panels ── */
.panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  margin-top: 20px;
  padding: 20px 20px;
  border-radius: 14px;
  background: #1e1e1e;
  border: 1px solid #333;
  width: 100%;
  max-width: 100%;
}

.input-label {
  font-size: clamp(0.7rem, 2.2vw, 0.85rem);
  color: #888;
  align-self: flex-start;
}

.input-row {
  display: flex;
  gap: 8px;
  width: 100%;
}

.url-input {
  flex: 1;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid #444;
  background: #2a2a2a;
  color: #ddd;
  font-size: clamp(0.8rem, 2.4vw, 0.95rem);
  outline: none;
  transition: border-color 0.2s;
}
.url-input:focus {
  border-color: #5a9;
}

.hint {
  font-size: clamp(0.6rem, 1.9vw, 0.75rem);
  color: #555;
  margin: 0;
}

/* ── Matching spinner ── */
.spinner {
  width: 36px;
  height: 36px;
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
  font-size: clamp(0.85rem, 2.6vw, 1rem);
  color: #ccc;
  text-align: center;
}

.match-buttons {
  display: flex;
  gap: 8px;
}

/* ── Buttons ── */
.action-btn {
  padding: 8px 18px;
  font-size: clamp(0.8rem, 2.4vw, 0.95rem);
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

/* ── Bottom bar ── */
.bottom-bar {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-top: 10px;
}

.error {
  color: #e05555;
  font-size: clamp(0.7rem, 2.2vw, 0.85rem);
  margin-top: 6px;
  text-align: center;
}
</style>
