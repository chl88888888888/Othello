import { createRouter, createWebHistory } from "vue-router";
import HomeView from "../views/HomeView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: HomeView,
    },
    {
      path: "/ai-battle",
      name: "ai-battle",
      component: () => import("../views/AIBattleView.vue"),
    },
    {
      path: "/online-battle",
      name: "online-battle",
      component: () => import("../views/OnlineBattleView.vue"),
    },
    {
      path: "/history",
      name: "history",
      component: () => import("../views/HistoryView.vue"),
    },
  ],
});

export default router;
