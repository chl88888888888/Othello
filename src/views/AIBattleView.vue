<script setup lang="ts">
import { ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useOthello } from "../composables/useOthello";

const router = useRouter();

const {
  blackScore,
  whiteScore,
  gameOver,
  currentTurn,
  errorMsg,
  flipAnim,
  isAiThinking,
  playerSide,
  canvasRef,
  startGame,
  handleClick,
  requestAiMove,
  turnLabel,
  winnerLabel,
  saveCurrentGame,
} = useOthello();

// ── 选边状态 ──
const showSidePicker = ref(true);
const pickingSide = ref(false);

function chooseSide(side: "black" | "white") {
  playerSide.value = side;
  showSidePicker.value = false;
  pickingSide.value = false;
  startGame();
}

function showPicker() {
  showSidePicker.value = true;
}

// ── AI 自动落子 ──
let aiTimer: ReturnType<typeof setTimeout> | null = null;

watch([currentTurn, gameOver, flipAnim, playerSide, showSidePicker], async ([turn, over, anim, side, picker]) => {
  if (over || anim || picker) return;
  if (turn !== side && !isAiThinking.value) {
    // AI 的回合，延迟片刻后自动落子
    aiTimer = setTimeout(async () => {
      await requestAiMove();
    }, 400);
  }
});

// ── 人类落子（包装：阻止非己方回合点击）──
function onBoardClick(e: MouseEvent) {
  if (showSidePicker.value) return;
  if (currentTurn.value !== playerSide.value) return;
  if (isAiThinking.value) return;
  handleClick(e);
}

// ── 新游戏 ──
function newGame() {
  if (aiTimer) clearTimeout(aiTimer);
  showPicker();
}

// ── 对局结束时自动保存 ──
watch(gameOver, async (over) => {
  if (over) {
    await saveCurrentGame();
  }
});

function goBack() {
  if (aiTimer) clearTimeout(aiTimer);
  router.push("/");
}
</script>

<template>
  <div class="ai-root">
    <!-- 顶栏 -->
    <div class="top-bar">
      <button class="back-btn" @click="goBack">← 返回</button>
      <h1 class="page-title">人机对战</h1>
      <div class="spacer"></div>
    </div>

    <!-- 选边面板 -->
    <div class="side-picker" v-if="showSidePicker && !pickingSide">
      <p class="picker-label">选择你的执子方：</p>
      <div class="picker-buttons">
        <button class="side-btn black-btn" @click="chooseSide('black')">
          <span class="side-icon">⚫</span>
          <span>执黑先行</span>
        </button>
        <button class="side-btn white-btn" @click="chooseSide('white')">
          <span class="side-icon">⚪</span>
          <span>执白后手</span>
        </button>
      </div>
    </div>

    <!-- 状态栏 -->
    <div class="info-bar">
      <div class="score black-score">
        <span class="piece-icon">⚫</span>
        <span>黑方 <strong>{{ blackScore }}</strong> 子</span>
      </div>

      <div class="turn-indicator" v-if="!gameOver && !showSidePicker">
        <template v-if="isAiThinking">
          AI 思考中...
        </template>
        <template v-else>
          轮到：<strong>{{ turnLabel() }}</strong>
          <span class="role-tag">{{ currentTurn === playerSide ? '(你)' : '(AI)' }}</span>
        </template>
      </div>
      <div class="turn-indicator winner" v-else-if="gameOver">
        {{ winnerLabel() }}
      </div>
      <div class="turn-indicator" v-else>
        请选择执子方
      </div>

      <div class="score white-score">
        <span class="piece-icon">⚪</span>
        <span>白方 <strong>{{ whiteScore }}</strong> 子</span>
      </div>
    </div>

    <!-- 棋盘 -->
    <canvas
      ref="canvasRef"
      class="board-canvas"
      :class="{ 'ai-turn': currentTurn !== playerSide && !gameOver && !showSidePicker }"
      @click="onBoardClick"
    ></canvas>

    <!-- 底部 -->
    <div class="bottom-bar">
      <p class="error" v-if="errorMsg">{{ errorMsg }}</p>
      <div class="bottom-buttons">
        <button class="new-game-btn" @click="newGame">新游戏</button>
        <button
          v-if="!showSidePicker"
          class="switch-btn"
          @click="showPicker"
        >换边</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ai-root {
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

/* ── 选边面板 ── */
.side-picker {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 20px 16px;
  margin-bottom: 8px;
  background: #2a2a2a;
  border-radius: 12px;
  width: 100%;
  max-width: 100%;
}

.picker-label {
  font-size: clamp(0.85rem, 3vw, 1.05rem);
  color: #ccc;
  text-align: center;
}

.picker-buttons {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  justify-content: center;
}

.side-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 20px;
  border: 2px solid #444;
  border-radius: 12px;
  background: #333;
  color: #f0f0f0;
  cursor: pointer;
  transition: all 0.2s;
  font-size: clamp(0.82rem, 2.8vw, 1rem);
}

.side-btn:hover {
  border-color: #888;
  background: #3a3a3a;
}

.black-btn:hover {
  border-color: #666;
  box-shadow: 0 0 12px rgba(0, 0, 0, 0.4);
}

.white-btn:hover {
  border-color: #aaa;
  box-shadow: 0 0 12px rgba(255, 255, 255, 0.15);
}

.side-icon {
  font-size: 1.8rem;
}

/* ── 信息栏 ── */
.info-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  max-width: 100%;
  margin-bottom: 8px;
  gap: 6px;
  flex-wrap: wrap;
}

.score {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: clamp(0.75rem, 2.5vw, 0.95rem);
  padding: 4px 10px;
  border-radius: 8px;
  background: #2a2a2a;
  white-space: nowrap;
}

.piece-icon {
  font-size: clamp(0.9rem, 3vw, 1.2rem);
}

.turn-indicator {
  font-size: clamp(0.8rem, 2.8vw, 1.05rem);
  padding: 4px 10px;
  border-radius: 8px;
  background: #333;
  white-space: nowrap;
}

.turn-indicator.winner {
  background: #4a3a0a;
  color: #ffd700;
}

.role-tag {
  font-size: 0.8rem;
  color: #888;
  margin-left: 4px;
}

/* ── 棋盘 ── */
.board-canvas {
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  cursor: pointer;
  transition: box-shadow 0.3s;
  max-width: 100%;
  height: auto;
}

.board-canvas.ai-turn {
  cursor: default;
  box-shadow: 0 4px 20px rgba(100, 140, 255, 0.25);
}

/* ── 底部 ── */
.bottom-bar {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.error {
  color: #f87171;
  font-size: clamp(0.75rem, 2.3vw, 0.9rem);
  text-align: center;
}

.bottom-buttons {
  display: flex;
  gap: 8px;
}

.new-game-btn {
  padding: 6px 18px;
  font-size: clamp(0.82rem, 2.6vw, 1rem);
  border: none;
  border-radius: 8px;
  background: #3a6b4c;
  color: #f0f0f0;
  cursor: pointer;
  transition: background 0.2s;
}

.new-game-btn:hover {
  background: #4a8b5c;
}

.switch-btn {
  padding: 6px 14px;
  font-size: clamp(0.78rem, 2.4vw, 0.95rem);
  border: 1px solid #555;
  border-radius: 8px;
  background: #2a2a2a;
  color: #ccc;
  cursor: pointer;
  transition: all 0.2s;
}

.switch-btn:hover {
  background: #3a3a3a;
  border-color: #777;
}
</style>

