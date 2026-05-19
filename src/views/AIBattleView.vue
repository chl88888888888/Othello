<script setup lang="ts">
import { useRouter } from "vue-router";
import { useOthello } from "../composables/useOthello";

const router = useRouter();

const {
  blackScore,
  whiteScore,
  gameOver,
  errorMsg,
  canvasRef,
  startGame,
  handleClick,
  turnLabel,
  winnerLabel,
} = useOthello();

function goBack() {
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

    <!-- 状态栏 -->
    <div class="info-bar">
      <div class="score black-score">
        <span class="piece-icon">⚫</span>
        <span>黑方 <strong>{{ blackScore }}</strong> 子</span>
      </div>

      <div class="turn-indicator" v-if="!gameOver">
        轮到：<strong>{{ turnLabel() }}</strong>
      </div>
      <div class="turn-indicator winner" v-else>
        {{ winnerLabel() }}
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
      @click="handleClick"
    ></canvas>

    <!-- 底部 -->
    <div class="bottom-bar">
      <p class="error" v-if="errorMsg">{{ errorMsg }}</p>
      <button class="new-game-btn" @click="startGame">🔄 新游戏</button>
    </div>
  </div>
</template>

<style scoped>
.ai-root {
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
  max-width: 480px;
  margin-bottom: 10px;
  gap: 10px;
}

.score {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.95rem;
  padding: 6px 14px;
  border-radius: 8px;
  background: #2a2a2a;
}

.piece-icon {
  font-size: 1.2rem;
}

.turn-indicator {
  font-size: 1.05rem;
  padding: 6px 16px;
  border-radius: 8px;
  background: #333;
  white-space: nowrap;
}

.turn-indicator.winner {
  background: #4a3a0a;
  color: #ffd700;
}

/* ── 棋盘 ── */
.board-canvas {
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  cursor: pointer;
}

/* ── 底部 ── */
.bottom-bar {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.error {
  color: #f87171;
  font-size: 0.9rem;
}

.new-game-btn {
  padding: 8px 24px;
  font-size: 1rem;
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
</style>
