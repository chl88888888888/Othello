# 黑白棋 Othello

一个基于 Tauri v2 的黑白棋（Reversi）桌面与移动端应用
使用 Vue 3 构建前端界面 Rust 驱动核心逻辑与 AI

## 功能特性

### 本地对弈
双人轮流在同一设备上对战
实时显示合法落子位置与双方棋子数量
流畅的棋子翻转动画

### AI 对战
搭载 AlphaZero 风格的 CNN 残差网络模型
使用 candle-core 进行本地推理 无需联网
模型文件位于 resources/othello_model safetensors

### 在线对战
基于 WebSocket 的实时联机功能
支持房间匹配与远程对弈
使用 tokio-tungstenite 实现高效通信

### 对局历史
所有对局自动保存至 SQLite 数据库
支持查看历史对局列表与胜负统计
可回放任意历史对局的完整走法过程

### 跨平台
支持 Windows / macOS / Linux 桌面平台
支持 Android 移动平台
使用 Tauri v2 统一构建

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri v2 |
| 前端 | Vue 3 + TypeScript + Vite |
| 路由 | Vue Router 4 |
| 后端语言 | Rust |
| 神经网络 | candle-core + candle-nn |
| 数据库 | SQLite (rusqlite) |
| 网络通信 | tokio-tungstenite (WebSocket) |
| 序列化 | serde + serde_json |
| 日志 | simplelog |

## 项目结构

```
Othello/
├── index html              入口 HTML
├── package json            前端依赖与脚本
├── vite config ts          Vite 构建配置
├── tsconfig json           TypeScript 配置
├── src/                    前端源码 (Vue 3)
│   ├── main ts             应用入口
│   ├── App vue             根组件
│   ├── views/              页面视图
│   │   ├── HomeView vue        主菜单
│   │   ├── AIBattleView vue    AI 对战
│   │   ├── OnlineBattleView vue 在线对战
│   │   └── HistoryView vue     历史记录
│   ├── composables/        组合式逻辑
│   │   ├── useOthello ts       游戏主逻辑
│   │   └── othello/            子模块
│   │       ├── types ts            类型定义
│   │       ├── constants ts        常量
│   │       ├── helpers ts          工具函数
│   │       ├── renderer ts         Canvas 渲染
│   │       └── animation ts       动画控制
│   ├── router/             路由配置
│   └── styles/             样式文件
├── src-tauri/              后端源码 (Rust)
│   ├── Cargo toml          Rust 依赖
│   ├── tauri conf json     Tauri 配置
│   ├── resources/          模型资源
│   │   └── othello_model safetensors  AI 模型权重
│   ├── src/
│   │   ├── main rs         入口
│   │   ├── lib rs          命令注册与调度
│   │   ├── game_logic rs   位棋盘核心算法
│   │   ├── ai rs           CNN 模型加载与推理
│   │   ├── db rs           SQLite 数据库操作
│   │   ├── network rs      WebSocket 联机模块
│   │   └── response rs     响应数据结构
│   └── gen/android/        Android 构建配置
└── public/                 静态资源
```

## 环境要求

- Node js 18+ 与 npm
- Rust 工具链 (rustc cargo)
- Tauri CLI (可选 也可通过 npm 调用)
- Android SDK 与 NDK (仅 Android 构建时需要)

## 快速开始

### 安装依赖

```bash
npm install
```

### 开发模式 (桌面端)

```bash
npm run tauri dev
```

这将启动 Vite 开发服务器并在桌面窗口中打开应用

### 构建桌面端

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`

### 构建 Android 端

```bash
npm run tauri android build
```

APK 输出路径为 `src-tauri/gen/android/app/build/outputs/apk/`

## 游戏规则

黑白棋（奥赛罗）在 8×8 的棋盘上进行
双方轮流落子 黑先白后
落子时必须夹住对方至少一枚棋子
被夹住的棋子会翻转成己方颜色
如果一方无法合法落子 则跳过回合
双方都无法落子时游戏结束
棋盘上棋子多的一方获胜

## 架构说明

### 位棋盘 (Bitboard)

游戏状态使用 64 位无符号整数表示棋盘
每一位代表棋盘上的一个格子
使用 Hyperbola Quintessence 算法计算落子翻转
位运算实现高效移动生成与合法性检测

### AI 模型

模型架构为 AlphaZero 风格残差网络
输入 2×8×8 (黑棋位棋盘 + 白棋位棋盘)
Conv2d(2→64 3×3) → ReLU
ResBlock ×3 (Conv2d 64→64 3×3 + skip connection)
Conv2d(64→1 1×1) → Flatten → Linear(64→1) → tanh
输出评估值 选择最有利的落子位置

### 数据库

使用 SQLite 嵌入式数据库
存储每局对弈的完整走法记录
支持按时间查询与胜负统计

## 命令参考

| 命令 | 说明 |
|------|------|
| `start_game` | 初始化新游戏 |
| `make_move` | 执行一步落子 |
| `ai_move` | AI 计算并返回落子位置 |
| `get_history` | 获取历史对局列表 |
| `get_game_detail` | 获取指定对局的走法详情 |
| `get_game_stats` | 获取总胜负统计 |
| `delete_history` | 删除指定对局记录 |
