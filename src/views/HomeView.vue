<script setup lang="ts">
import { useRouter } from "vue-router";

const router = useRouter();

interface MenuItem {
  title: string;
  subtitle: string;
  route: string;
  enabled: boolean;
}

const menus: MenuItem[] = [
  {
    title: "人机对战",
    subtitle: "与 AI 进行黑白棋对弈",
    route: "/ai-battle",
    enabled: true,
  },
  {
    title: "联网对战",
    subtitle: "与其他玩家在线对战",
    route: "/online-battle",
    enabled: true,
  },
  {
    title: "历史查询",
    subtitle: "查看历史对局记录",
    route: "/history",
    enabled: true,
  },
];

function navigate(menu: MenuItem) {
  if (menu.enabled) {
    router.push(menu.route);
  }
}
</script>

<template>
  <div class="home-root">
    <div class="home-hero">
      <h1 class="home-title">♟️ 黑白棋</h1>
      <p class="home-subtitle">Othello / Reversi</p>
    </div>

    <div class="menu-grid">
      <button
        v-for="menu in menus"
        :key="menu.route"
        class="menu-card"
        :class="{ disabled: !menu.enabled }"
        @click="navigate(menu)"
      >
        <span class="menu-title">{{ menu.title }}</span>
        <span class="menu-sub">{{ menu.subtitle }}</span>
      </button>
    </div>

    <footer class="home-footer">
      <p>v0.1.0 — 基于 Tauri + Vue 3 构建</p>
    </footer>
  </div>
</template>

<style scoped>
.home-root {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  min-height: 100dvh;
  padding: 32px 16px;
  gap: 28px;
  width: 100%;
  max-width: 100vw;
  overflow-x: hidden;
}

.home-hero {
  text-align: center;
}

.home-title {
  font-size: clamp(2rem, 10vw, 3rem);
  letter-spacing: 4px;
  margin-bottom: 4px;
  background: linear-gradient(135deg, #e0e0e0, #a0c0a0);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.home-subtitle {
  font-size: clamp(0.8rem, 3vw, 1rem);
  color: #888;
  letter-spacing: 3px;
}

/* ── 菜单卡片网格 ── */
.menu-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
  max-width: 100%;
}

.menu-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 18px 16px;
  border: 1px solid #3a3a3a;
  border-radius: 14px;
  background: #262626;
  color: #e0e0e0;
  cursor: pointer;
  transition: all 0.25s ease;
}

.menu-card:hover {
  border-color: #5a8a5a;
  background: #2a302a;
  transform: translateY(-2px);
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.4);
}

.menu-card:active {
  transform: translateY(0);
}

.menu-card.disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.menu-card.disabled:hover {
  border-color: #3a3a3a;
  background: #262626;
  transform: none;
  box-shadow: none;
}

.menu-icon {
  font-size: 2.4rem;
}

.menu-title {
  font-size: clamp(1rem, 3.5vw, 1.2rem);
  font-weight: 600;
}

.menu-sub {
  font-size: clamp(0.7rem, 2.2vw, 0.82rem);
  color: #999;
}

/* ── 底部 ── */
.home-footer {
  text-align: center;
}

.home-footer p {
  font-size: clamp(0.65rem, 2vw, 0.78rem);
  color: #555;
}
</style>
