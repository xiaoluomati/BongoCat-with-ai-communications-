# 3D 模型桌宠 — 详细设计文档

> **分支**: feature/new3d
> **日期**: 2026-06-08
> **状态**: 设计阶段

---

## 一、概述

在现有 BongoCat 项目基础上，新增一个独立窗口展示 3D 角色模型，提供"呼吸"和"自然运动"效果。与现有 Live2D 主窗口并存，用户可在设置中选择展示模式（2D / 3D）或同时展示。

### 1.1 目标

- 加载 GLTF/GLB/VRM/PMX(VMD) 格式的 3D 角色模型
- 模型具备呼吸效果（scale 正弦波）
- 模型具备自然 idle 运动（烘焙动画 + 程序化微动）
- 支持鼠标拖拽旋转、滚轮缩放
- 右键菜单与现有系统共享
- 独立的模型管理（上传、切换、删除）

### 1.2 非目标（本期不做）

- 模型物理碰撞
- 复杂光照/阴影/后处理
- 多人/多模型同屏
- 模型动画编辑器
- 面部捕捉/语音驱动嘴型

---

## 二、架构设计

### 2.1 窗口模型

新增一个 Tauri 窗口 `model3d`，与现有 5 个窗口并列：

```
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│  main    │ │preference│ │  chat    │ │compreh.. │ │ model3d  │ │  (现有)  │
│ Live2D   │ │  设置    │ │  聊天    │ │ 功能面板 │ │ 3D 模型  │ │          │
└──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘
   WebView     WebView      WebView       WebView         WebView
   PixiJS       Vue 3        Vue 3         Vue 3        Three.js
```

窗口特性：
- 无边框、透明背景
- 始终置顶（与主窗口同层级管理）
- 可拖拽移动
- 支持鼠标穿透模式（与主窗口一样）

### 2.2 技术栈

| 层 | 技术 | 用途 |
|------|------|------|
| 渲染 | Three.js 0.170+ | 3D 场景渲染 |
| 模型加载 | GLTFLoader / MMDLoader (内置) / @pixiv/three-vrm | 模型文件解析 |
| 动画 | THREE.AnimationMixer | 骨骼动画播放 |
| 前端框架 | Vue 3 (Composition API) | 页面组件 |
| 构建 | Vite | 打包 |
| 后端 | Rust / Tauri v2 | 命令、文件管理 |

### 2.3 新增依赖

```json
{
  "three": "^0.170.0",
  "@pixiv/three-vrm": "^3.0.0"
}
```

three.js core 约 150KB gzipped，VRM loader 约 10KB。

MMDLoader 已内置于 `three/examples/jsm/loaders/MMDLoader.js`，随 three 包安装，无需额外依赖。MMD 物理模拟需要 `ammo.js`（约 1.5MB），但桌宠场景可关闭物理仅使用骨骼动画。

### 2.4 模型格式支持

| 格式 | 来源 | 加载器 | 动画 | 备注 |
|------|------|--------|------|------|
| GLTF/GLB | Sketchfab, Blender 导出 | GLTFLoader (内置) | ✅ | 最通用 |
| VRM 1.0 | VRoid Studio | @pixiv/three-vrm | ✅ | 角色专用，内置表情 |
| PMX + VMD | 模之屋, BowlRoll | MMDLoader (内置) | ✅ | MMD 生态，资源最丰富 |
| PMX (仅模型) | 模之屋 | MMDLoader (内置) | ❌ | 配合 procedural 动画使用 |

---

## 三、前端设计

### 3.1 目录结构

```
src/
├── pages/
│   └── model3d/
│       └── index.vue              # 3D 展示主页面
├── composables/
│   └── useModel3D.ts              # Three.js 场景管理 composable
├── stores/
│   └── model3d.ts                 # 3D 模型 Pinia store
└── utils/
    └── model3d/
        ├── scene.ts               # 场景初始化
        ├── loader.ts              # 模型加载器
        ├── animation.ts           # 动画管理（呼吸 + 自然运动）
        └── interaction.ts         # 鼠标交互
```

### 3.2 组件树

```
model3d/index.vue
├── <canvas id="threeCanvas">       # Three.js 渲染目标
├── 加载状态提示（loading spinner）
├── 错误状态提示（模型加载失败）
└── 空状态提示（未配置模型）
```

### 3.3 useModel3D Composable — 核心逻辑

```typescript
// composables/useModel3D.ts

export function useModel3D() {
  // ── 状态 ──
  const scene: THREE.Scene
  const camera: THREE.PerspectiveCamera
  const renderer: THREE.WebGLRenderer
  const model: THREE.Group | null
  const mixer: THREE.AnimationMixer | null
  const isLoading: Ref<boolean>
  const error: Ref<string | null>

  // ── 方法 ──
  async function init(canvas: HTMLCanvasElement): Promise<void>
  async function loadModel(path: string): Promise<void>
  function unloadModel(): void
  function destroy(): void
  function resize(width: number, height: number): void

  // ── 动画 ──
  function startBreathe(intensity?: number): void
  function stopBreathe(): void
  function playIdleAnimation(): void
  function playGesture(name: string): void
}
```

### 3.4 场景结构

```
Scene
├── AmbientLight (color: 0xffffff, intensity: 0.8)
├── DirectionalLight (color: 0xffffff, intensity: 0.6)
├── Model Group (根节点)
│   ├── Mesh (身体)
│   ├── Mesh (头部)
│   ├── SkinnedMesh (蒙皮网格)
│   └── Bone hierarchy (骨骼层级)
└── (可选) Ground plane 或 reference grid (调试用)
```

### 3.5 呼吸效果实现

```typescript
// 正弦波驱动 model.scale，叠加在 idle 动画上
function startBreathe(intensity = 0.02, period = 3.0) {
  const baseScale = model.scale.clone()
  
  // 在 render loop 中:
  const t = performance.now() / 1000
  const breathe = 1 + Math.sin(t * (2 * Math.PI / period)) * intensity
  model.scale.setScalar(baseScale.x * breathe)
}
```

### 3.6 自然运动实现

分层叠加：

```
Layer 1: 骨骼动画 (AnimationMixer)
  └─ 模型自带的 idle animation clip 循环播放

Layer 2: 程序化微动 (procedural noise)
  └─ 低频 Perlin noise 驱动 model.rotation (整体微摆)
  └─ 高频 noise 驱动头部骨骼 rotation (头部微动)

Layer 3: 随机 gesture
  └─ 每 5-15 秒随机触发: 眨眼、摆头、挥手
  └─ 从模型 animation clips 中选择
```

```typescript
// animation.ts — 程序化微动
function applyProceduralMotion(model: THREE.Group, clock: THREE.Clock) {
  const t = clock.getElapsedTime()
  
  // 身体微晃
  model.rotation.z = Math.sin(t * 0.7) * 0.03
  model.rotation.y = Math.sin(t * 0.5 + 1) * 0.02
  
  // 轻微上下浮动
  model.position.y = Math.sin(t * 0.6) * 0.005
}
```

### 3.7 鼠标交互

```
左键拖拽     → 模型水平旋转 (orbit around Y axis)
滚轮         → 缩放 (调整相机距离 / fov)
右键         → 弹出共享菜单 (与现有 useSharedMenu 复用)
Shift+右键拖拽 → 窗口移动 (与现有主窗口行为一致)
```

使用 Three.js `OrbitControls` 或自定义实现（推荐自定义，更轻量）。

### 3.8 页面状态

```
States:
  empty    — 未配置模型 → 显示引导提示"请上传 3D 模型"
  loading  — 正在加载    → 显示 spinner + 进度
  ready    — 加载完成    → 展示模型 + 动画
  error    — 加载失败    → 显示错误信息 + 重试按钮
```

---

## 四、后端设计

### 4.1 Rust 命令

```
模型管理:
  list_3d_models()         → Vec<ModelInfo>  列出可用模型
  load_3d_model(id)        → ModelPath        获取模型文件路径
  delete_3d_model(id)      → ()               删除模型
  save_3d_model_config()   → ()               保存 3D 配置

配置（扩展现有 config）:
  Model3DConfig {
    enabled: bool               // 是否启用 3D 窗口
    current_model_id: String    // 当前模型 ID
    auto_breathe: bool          // 自动呼吸
    idle_animation: bool        // idle 动画
    window_scale: f32           // 窗口缩放
  }
```

### 4.2 文件存储

```
app_data/
├── models_3d/                    # 3D 模型存储
│   ├── index.json                # 模型索引
│   └── {model_id}/
│       ├── model.glb             # 模型主文件
│       ├── textures/             # 纹理
│       └── meta.json             # 模型元数据
```

`index.json` 结构：
```json
[
  {
    "id": "a1b2c3d4",
    "name": "我的角色",
    "format": "pmx",
    "file_path": "models_3d/a1b2c3d4/model.pmx",
    "motion_paths": ["motions/idle.vmd", "motions/wave.vmd"],
    "added_at": "2026-06-08T12:00:00Z"
  }
]
```

### 4.3 tauri.conf.json 窗口配置

```json
{
  "windows": [
    // ... 现有窗口 ...
    {
      "label": "model3d",
      "title": "3D Model",
      "url": "/#/model3d",
      "width": 400,
      "height": 500,
      "decorations": false,
      "transparent": true,
      "alwaysOnTop": true,
      "resizable": true,
      "visible": false
    }
  ]
}
```

---

## 五、Store 设计

### 5.1 model3d Store (Pinia)

```typescript
// stores/model3d.ts

export const useModel3DStore = defineStore('model3d', () => {
  // 状态
  const enabled = ref(false)               // 3D 窗口是否启用
  const currentModelId = ref<string | null>(null)
  const models = ref<Model3DInfo[]>([])     // 可用模型列表
  const windowScale = ref(100)              // 窗口缩放 %
  const autoBreathe = ref(true)             // 自动呼吸
  const idleAnimation = ref(true)           // idle 动画

  // 方法
  async function init()
  async function loadModels()
  async function switchModel(id: string)
  async function deleteModel(id: string)
  async function uploadModel(path: string, name: string)
  
  return { enabled, currentModelId, models, windowScale,
           autoBreathe, idleAnimation,
           init, loadModels, switchModel, deleteModel, uploadModel }
})
```

### 5.2 与现有 Store 的关系

```
configStore (现有)
  ├─ currentCharacterId
  ├─ llmConfig
  └─ ttsConfig
       └─ (新增) model3dConfig { enabled, current_model_id }

model3dStore (新增)
  └─ 读写 configStore.model3dConfig
```

---

## 六、路由设计

```typescript
// router/index.ts 新增
{
  path: '/model3d',
  name: 'model3d',
  component: () => import('@/pages/model3d/index.vue'),
}
```

---

## 七、与现有功能联动

| 现有功能 | 3D 模型联动 |
|----------|------------|
| 右键菜单 (useSharedMenu) | 复用，增删模型操作 |
| 表情触发 (LLM emotion) | 可选：触发 3D 模型对应表情 blendshape |
| TTS 播放 | 可选：嘴型同步（本期不做） |
| 键盘/手柄交互 | 可选：触发特定 gesture |
| 窗口跟随 (window_follower) | 可选：3D 窗口跟随主窗口 |

---

## 八、Config 扩展示例

```json
{
  "model3d": {
    "enabled": true,
    "current_model_id": "a1b2c3d4",
    "window_scale": 100,
    "auto_breathe": true,
    "idle_animation": true,
    "window_position": { "x": 1200, "y": 300 },
    "window_size": { "width": 400, "height": 500 }
  }
}
```

---

## 九、测试要点

1. 模型加载：PMX、GLB、VRM 格式各一个测试模型
2. 呼吸效果：可见的缩放脉冲
3. Idle 动画：骨骼动画正常循环
4. 鼠标交互：旋转、缩放、拖拽移动
5. 窗口管理：显示/隐藏/置顶与现有窗口不冲突
6. 模型切换：切换无闪烁，旧模型正确销毁
7. 性能：60fps 稳定，内存无泄漏
8. 无模型状态：空状态提示正常

---

## 十、关键决策记录

| 决策 | 结论 |
|------|------|
| 3D 库 | Three.js（而非 Babylon.js 或 PixiJS 伪 3D） |
| 新窗口 vs 复用 | 独立新窗口（而非复用主窗口） |
| 动画方案 | 烘焙动画 + procedural noise 混合（而非纯程序化） |
| 模型格式 | GLB、VRM、PMX(VMD) 均支持，使用对应的内置加载器 |
| 相机控制 | 自定义实现（轻量），不引入 OrbitControls |
| 配置存储 | 扩展现有 config.json，新增 model3d 段 |

---

**版本**: 1.0
**更新**: 2026-06-08
