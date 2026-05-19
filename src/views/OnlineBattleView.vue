<script setup lang="ts">
import { useRouter } from "vue-router";

const router = useRouter();

function goBack() {
  router.push("/");
}
</script>

<template>
  <div class="online-root">
    <!-- 顶栏 -->
    <div class="top-bar">
      <button class="back-btn" @click="goBack">← 返回</button>
      <h1 class="page-title">🌐 联网对战</h1>
      <div class="spacer"></div>
    </div>

    <!-- 占位内容 -->
    <div class="placeholder">
      <span class="placeholder-icon">🔧</span>
      <h2>功能开发中</h2>
      <p>
        联网对战功能将在后续版本中实现。<br />
        届时您将可以与其他玩家在线对弈。
      </p>
      <div class="feature-list">
        <div class="feature-item">
          <span class="check">✓</span> 房间创建与加入
        </div>
        <div class="feature-item">
          <span class="check">✓</span> 实时对局同步
        </div>
        <div class="feature-item">
          <span class="check">✓</span> 对手匹配系统
        </div>
        <div class="feature-item">
          <span class="check">✓</span> 聊天与互动
        </div>
      </div>
    </div>
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

/* ── 占位内容 ── */
.placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 40px 20px;
  gap: 12px;
  min-height: 400px;
}

.placeholder-icon {
  font-size: 3.5rem;
  margin-bottom: 8px;
}

.placeholder h2 {
  font-size: 1.4rem;
  color: #ccc;
}

.placeholder p {
  color: #888;
  line-height: 1.7;
  max-width: 320px;
}

/* ── 功能列表 ── */
.feature-list {
  margin-top: 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  text-align: left;
}

.feature-item {
  display: flex;
  align-items: center;
  gap: 10px;
  color: #aaa;
  font-size: 0.9rem;
}

.check {
  color: #5a8a5a;
  font-weight: bold;
}
</style>
