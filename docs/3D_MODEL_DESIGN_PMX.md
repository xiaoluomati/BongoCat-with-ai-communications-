# 3D 桌宠 PMX 版本 — 详细设计

> **分支**: feature/new3d
> **日期**: 2026-06-08
> **范围**: 仅 PMX/VMD (MMD) 格式，暂不涉及 GLB/VRM

---

## 一、概述

基于 Three.js MMDLoader，加载模之屋下载的 PMX 模型和 VMD 动作文件，实现 3D 角色桌宠展示。

### 1.1 用户已有的资源

```
D:\文档\ai女友\
  ├── 凯茜娅-狂诗_by_神帝宇_xxx.zip         (PMX + 贴图)
  ├── 凯茜娅-朝翼_by_神帝宇_xxx.zip         (PMX + 贴图)
  ├── 沐浪倦姿_by_nnishishei123_xxx.zip    (PMX + 贴图)
  └── ...
```

这些是模之屋的标准 MMD 模型 zip 包，解压后通常包含：`.pmx` 模型文件、`textures/` 贴图文件夹。

---

## 二、MMD 模型文件结构

### 2.1 典型 PMX 模型包解压后

```
凯茜娅-狂诗/
├── 凯茜娅-狂诗.pmx          # 模型主文件（顶点、骨骼、蒙皮、材质、表情 morph）
├── tex/                      # 贴图（或 textures/ 或直接放在根目录）
│   ├── face.png
│   ├── body.png
│   ├── hair.png
│   └── ...
├── 凯茜娅-狂诗_ idle.vmd     # （可选）idle 动作
├── 凯茜娅-狂诗_ wave.vmd     # （可选）其他动作
└── readme.txt                # （可选）作者说明
```

### 2.2 PMX 文件内部结构

```
PMX 文件:
├── 头部信息 (版本、编码、附加信息数)
├── 模型信息 (名称、注释)
├── 顶点数据 (位置、法线、UV、权重、变形偏移)
├── 面数据 (三角形索引)
├── 纹理列表 (相对路径指向贴图文件)
├── 材质数据 (漫反射、高光、环境色、toon 索引)
├── 骨骼数据 (名称、父子关系、变换、IK 链、物理附加)
├── 表情 morph (名称、类型、顶点偏移)
├── 刚体 (物理碰撞, MMD 引擎使用)
└── 关节 (物理约束, MMD 引擎使用)
```

对桌宠重要的：**骨骼**（做动画）、**表情 morph**（做表情变化）、**纹理**（渲染外观）。刚体和关节（物理）仅 MMD 物理引擎使用，桌宠场景可忽略。

### 2.3 VMD 动作文件

VMD 存储关键帧数据：
```
VMD 文件:
├── 骨骼关键帧 (bone_name → [frame: {translation, rotation}])
├── 表情关键帧 (morph_name → [frame: {weight}])
├── 相机关键帧 (略，桌宠不使用)
└── 其他 (略)
```

---

## 三、Three.js MMDLoader 工作方式

### 3.1 加载流程

```typescript
const loader = new MMDLoader()

// ① 加载 PMX 模型
const model = await loader.loadAsync(
  'path/to/model.pmx',          // PMX 文件路径
  null,                          // VMD 动作文件（可后续加载）
  null,                          // 相机 VMD（不需要）
  (progress) => { /* 进度 */ }
)

// ② 加载 VMD 动作（可选，也可后加载）
const motion = await loader.loadVMDAsync('path/to/idle.vmd')

// ③ 创建动画播放器
const mixer = new THREE.AnimationMixer(model)
const action = mixer.clipAction(motion)
action.play()
```

### 3.2 MMDLoader 内部做了什么

1. 解析 PMX 二进制 → 顶点/骨骼/材质/表情数据
2. 创建 `THREE.SkinnedMesh` 对象（三类：body / face / hair）
3. 构建骨骼层级 → `THREE.Bone` 树
4. 加载贴图 → 按 PMX 纹理列表中记录的相对路径查找
5. 解析 VMD → `THREE.AnimationClip`（骨骼关键帧 + 表情关键帧）

### 3.3 贴图路径解析

MMDLoader 在 PMX 所在目录下查找贴图。PMX 内部存储的是**相对路径**（如 `tex/face.png`），所以文件结构必须保持一致。如果贴图找不全，对应部位会显示为白色/灰色。

### 3.4 需要的 three.js 模块

```typescript
import * as THREE from 'three'
import { MMDLoader } from 'three/examples/jsm/loaders/MMDLoader.js'
// MMDLoader 内部依赖以下 experimental 模块（不需要手动导入）:
//   MMDPhysics (可选，需 ammo.js)
//   MMDAnimationHelper (可选，简化动画管理)
//   各种 shader (toon, outline 等)
```

---

## 四、前端设计

### 4.1 文件结构

```
src/
├── pages/
│   └── model3d/
│       └── index.vue                    # 3D 展示页面
├── composables/
│   └── useModel3D.ts                    # 场景 + MMD 模型管理
├── utils/
│   └── model3d/
│       ├── scene.ts                     # Three.js 场景初始化
│       ├── loader.ts                    # MMD 加载器封装
│       ├── animation.ts                 # 动画控制（呼吸 + 动作）
│       └── interaction.ts              # 鼠标交互
└── stores/
    └── model3d.ts                       # 模型配置 store
```

### 4.2 页面组件 `model3d/index.vue`

```vue
<template>
  <div class="model3d-container">
    <!-- 空状态 -->
    <div v-if="status === 'empty'" class="empty-hint">
      请选择模型
    </div>

    <!-- 加载中 -->
    <div v-if="status === 'loading'" class="loading-hint">
      加载中... {{ progress }}%
    </div>

    <!-- 错误 -->
    <div v-if="status === 'error'" class="error-hint">
      加载失败: {{ error }}
      <button @click="retry">重试</button>
    </div>

    <!-- 渲染画布 -->
    <canvas ref="canvasRef" id="model3dCanvas" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useModel3D } from '@/composables/useModel3D'
import { useModel3DStore } from '@/stores/model3d'

const store = useModel3DStore()
const { init, loadModel, destroy, status, progress, error } = useModel3D()

onMounted(async () => {
  await init(canvasRef.value)
  if (store.currentModelPath) {
    await loadModel(store.currentModelPath)
  }
})

watch(() => store.currentModelPath, async (path) => {
  if (path) await loadModel(path)
})

onUnmounted(() => destroy())
</script>
```

`status` 状态机：
```
empty ──→ loading ──→ ready
  ↑                     │
  └──←── error ←────────┘
```

### 4.3 场景初始化 `scene.ts`

```typescript
// 搭建最简场景：一个模型 + 基础光照
export function createScene(canvas: HTMLCanvasElement) {
  const renderer = new THREE.WebGLRenderer({
    canvas,
    alpha: true,                // 透明背景（与桌宠一致）
    antialias: true,
  })
  renderer.setPixelRatio(window.devicePixelRatio)

  const scene = new THREE.Scene()

  const camera = new THREE.PerspectiveCamera(
    45,                         // FOV
    canvas.width / canvas.height,
    0.1,
    100
  )
  camera.position.set(0, 1.5, 5) // 面对模型的合适位置

  // 基础光照（MMD 模型不需要复杂光照，toon 材质自发光成分高）
  const ambient = new THREE.AmbientLight(0xffffff, 0.8)
  scene.add(ambient)
  const directional = new THREE.DirectionalLight(0xffffff, 0.6)
  directional.position.set(1, 2, 3)
  scene.add(directional)

  return { renderer, scene, camera }
}
```

### 4.4 MMD 加载器封装 `loader.ts`

```typescript
import * as THREE from 'three'
import { MMDLoader } from 'three/examples/jsm/loaders/MMDLoader.js'

export class ModelLoader {
  private loader: MMDLoader

  constructor() {
    this.loader = new MMDLoader()
  }

  // 加载 PMX 模型（必须）
  async loadModel(
    modelPath: string,
    onProgress?: (pct: number) => void
  ): Promise<THREE.SkinnedMesh> {
    return this.loader.loadAsync(modelPath, null, null, (e) => {
      if (e.lengthComputable) {
        onProgress?.(Math.round((e.loaded / e.total) * 100))
      }
    })
  }

  // 加载 VMD 动作（可选）
  async loadMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    return this.loader.loadVMDAsync(vmdPath)
  }

  // 加载相机 VMD（桌宠场景不使用，但保留接口）
  async loadCameraMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    return this.loader.loadVMDAsync(vmdPath, true)
  }
}
```

### 4.5 动画控制 `animation.ts`

```typescript
import * as THREE from 'three'

export interface AnimationState {
  // 呼吸
  breatheEnabled: boolean
  breatheIntensity: number    // 默认 0.02
  breathePeriod: number       // 默认 3.0 秒

  // 骨骼动作
  mixer: THREE.AnimationMixer | null
  currentAction: THREE.AnimationAction | null
  actionMap: Map<string, THREE.AnimationClip>

  // 程序化微动
  proceduralEnabled: boolean
}

// ── 呼吸效果 ──
export function applyBreathe(
  model: THREE.Object3D,
  state: AnimationState,
  time: number
) {
  if (!state.breatheEnabled) return
  const breathe = 1 + Math.sin(time * (2 * Math.PI / state.breathePeriod)) * state.breatheIntensity
  model.scale.setScalar(breathe)
}

// ── 骨骼动作切换 ──
export function playMotion(
  state: AnimationState,
  name: string,
  fadeIn = 0.5
) {
  const clip = state.actionMap.get(name)
  if (!clip || !state.mixer) return

  const newAction = state.mixer.clipAction(clip)
  if (state.currentAction) {
    newAction.crossFadeFrom(state.currentAction, fadeIn, true)
  }
  newAction.play()
  state.currentAction = newAction
}

// ── 程序化微动 ──
export function applyProceduralMotion(
  model: THREE.Object3D,
  time: number
) {
  // 身体微晃
  model.rotation.z = Math.sin(time * 0.7) * 0.03
  model.rotation.y = Math.sin(time * 0.5 + 1) * 0.02

  // 轻微上下浮动
  model.position.y += Math.sin(time * 0.6) * 0.003
}

// ── Render loop ──
export function animate(
  state: AnimationState,
  model: THREE.Object3D,
  renderer: THREE.WebGLRenderer,
  scene: THREE.Scene,
  camera: THREE.Camera,
  clock: THREE.Clock
) {
  const delta = clock.getDelta()
  const elapsed = clock.getElapsedTime()

  // 1. 更新骨骼动画
  state.mixer?.update(delta)

  // 2. 呼吸
  applyBreathe(model, state, elapsed)

  // 3. 程序化微动（如果没播放骨骼动作）
  if (state.proceduralEnabled && !state.currentAction?.isRunning()) {
    applyProceduralMotion(model, elapsed)
  }

  // 4. 渲染
  renderer.render(scene, camera)
  requestAnimationFrame(() => animate(state, model, renderer, scene, camera, clock))
}
```

### 4.6 鼠标交互 `interaction.ts`

```typescript
// 轻量交互，不用 OrbitControls（避免额外依赖和复杂手势冲突）

export class ModelInteraction {
  private isDragging = false
  private prevX = 0
  private prevY = 0
  private rotateY = 0       // 模型 Y 轴旋转角度
  private cameraDistance = 5 // 相机距离

  constructor(
    private model: THREE.Object3D,
    private camera: THREE.PerspectiveCamera,
    private canvas: HTMLCanvasElement
  ) {
    canvas.addEventListener('mousedown', this.onMouseDown)
    canvas.addEventListener('mousemove', this.onMouseMove)
    canvas.addEventListener('mouseup', this.onMouseUp)
    canvas.addEventListener('wheel', this.onWheel)
  }

  private onMouseDown = (e: MouseEvent) => {
    if (e.buttons === 1) {    // 左键拖拽
      this.isDragging = true
      this.prevX = e.clientX
    }
  }

  private onMouseMove = (e: MouseEvent) => {
    if (!this.isDragging) return
    const dx = e.clientX - this.prevX
    this.rotateY += dx * 0.01
    this.model.rotation.y = this.rotateY
    this.prevX = e.clientX
  }

  private onMouseUp = () => {
    this.isDragging = false
  }

  private onWheel = (e: WheelEvent) => {
    e.preventDefault()
    this.cameraDistance += e.deltaY * 0.01
    this.cameraDistance = Math.max(2, Math.min(10, this.cameraDistance))
    this.camera.position.z = this.cameraDistance
  }

  destroy() {
    this.canvas.removeEventListener('mousedown', this.onMouseDown)
    this.canvas.removeEventListener('mousemove', this.onMouseMove)
    this.canvas.removeEventListener('mouseup', this.onMouseUp)
    this.canvas.removeEventListener('wheel', this.onWheel)
  }
}
```

### 4.7 useModel3D Composable

```typescript
// composables/useModel3D.ts — 将所有模块连接起来

export function useModel3D() {
  const store = useModel3DStore()
  const status = ref<'empty' | 'loading' | 'ready' | 'error'>('empty')
  const progress = ref(0)
  const error = ref<string | null>(null)

  let renderer: THREE.WebGLRenderer | null = null
  let scene: THREE.Scene | null = null
  let camera: THREE.PerspectiveCamera | null = null
  let model: THREE.SkinnedMesh | null = null
  let animState: AnimationState | null = null
  let interaction: ModelInteraction | null = null
  let clock: THREE.Clock | null = null

  const loader = new ModelLoader()

  async function init(canvas: HTMLCanvasElement) {
    const ctx = createScene(canvas)
    renderer = ctx.renderer
    scene = ctx.scene
    camera = ctx.camera
    clock = new THREE.Clock()
  }

  async function loadModel(modelPath: string) {
    if (!scene) return
    status.value = 'loading'
    error.value = null
    progress.value = 0

    try {
      // 卸载旧模型
      unloadModel()

      model = await loader.loadModel(modelPath, (pct) => {
        progress.value = pct
      })

      scene.add(model)

      // 初始化动画
      const mixer = new THREE.AnimationMixer(model)
      animState = createAnimationState(mixer)

      // 如果有 VMD 动作文件，加载并播放
      const motions = store.currentMotions
      for (const [name, vmdPath] of Object.entries(motions)) {
        const clip = await loader.loadMotion(vmdPath)
        animState.actionMap.set(name, clip)
        if (name === 'idle') playMotion(animState, 'idle')
      }

      // 开始 render loop
      requestAnimationFrame(() =>
        animate(animState!, model!, renderer!, scene!, camera!, clock!)
      )

      // 交互
      interaction?.destroy()
      interaction = new ModelInteraction(model!, camera!, renderer!.domElement)

      status.value = 'ready'
    } catch (e) {
      error.value = String(e)
      status.value = 'error'
    }
  }

  function unloadModel() {
    if (model) {
      scene?.remove(model)
      model = null
    }
    animState?.mixer?.stopAllAction()
    animState = null
    interaction?.destroy()
    interaction = null
  }

  function destroy() {
    unloadModel()
    renderer?.dispose()
  }

  return { init, loadModel, destroy, status, progress, error }
}
```

---

## 五、Store 设计

### 5.1 model3d Store

```typescript
// stores/model3d.ts

export interface Model3DInfo {
  id: string           // UUID
  name: string         // 显示名称
  pmxPath: string      // PMX 文件路径
  motions: Record<string, string>  // { "idle": "path/to/idle.vmd", ... }
  addedAt: string
}

export const useModel3DStore = defineStore('model3d', () => {
  const enabled = ref(false)
  const models = ref<Model3DInfo[]>([])
  const currentModelId = ref<string | null>(null)
  const windowScale = ref(100)
  const breatheEnabled = ref(true)

  // 辅助 getter
  const currentModel = computed(() =>
    models.value.find(m => m.id === currentModelId.value)
  )
  const currentPmxPath = computed(() => currentModel.value?.pmxPath ?? null)
  const currentMotions = computed(() => currentModel.value?.motions ?? {})

  async function loadModels() { /* invoke list_3d_models */ }
  async function addModel(name: string, pmxPath: string) { /* invoke */ }
  async function removeModel(id: string) { /* invoke */ }
  async function selectModel(id: string) { currentModelId.value = id }
  async function addMotion(modelId: string, name: string, vmdPath: string) { /* invoke */ }

  return { enabled, models, currentModelId, windowScale, breatheEnabled,
           currentModel, currentPmxPath, currentMotions,
           loadModels, addModel, removeModel, selectModel, addMotion }
})
```

---

## 六、后端设计

### 6.1 Rust 命令

```rust
// 模型管理
#[tauri::command]
fn list_3d_models() -> Vec<Model3DInfo> { /* 读 models_3d/index.json */ }

#[tauri::command]
fn add_3d_model(name: String, source_path: String) -> Model3DInfo {
  // 复制 PMX + 贴图到 app_data/models_3d/{id}/
  // 写入 index.json
}

#[tauri::command]
fn remove_3d_model(id: String) { /* 删除文件夹 + 更新 index.json */ }

#[tauri::command]
fn add_model_motion(model_id: String, name: String, vmd_path: String) {
  // 复制 VMD 到模型目录的 motions/ 子文件夹
}
```

### 6.2 文件存储

```
app_data/models_3d/
├── index.json
├── a1b2c3d4/                      # 凯茜娅-狂诗
│   ├── model.pmx
│   ├── tex/                       # 贴图（保持原始目录结构）
│   │   ├── face.png
│   │   └── body.png
│   └── motions/                   # VMD 动作文件
│       ├── idle.vmd
│       └── wave.vmd
├── e5f6g7h8/                      # 朝翼
│   └── ...
└── i9j0k1l2/                      # 沐浪倦姿
    └── ...
```

### 6.3 模型导入流程

```
用户在设置界面点击"导入模型"
  → 弹出文件选择器，选 .pmx 文件
  → Rust 端：
     1. 生成 UUID
     2. 创建 models_3d/{uuid}/ 目录
     3. 复制 PMX 到 models_3d/{uuid}/model.pmx
     4. 扫描源目录下的贴图文件，复制到 models_3d/{uuid}/tex/
        （保持 PMX 内记录的相对路径结构）
     5. 写入 index.json
  → 返回 Model3DInfo 给前端
```

### 6.4 tauri.conf.json 窗口注册

```json
{
  "label": "model3d",
  "title": "3D Model",
  "url": "/#/model3d",
  "width": 400,
  "height": 600,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "visible": false
}
```

---

## 七、关键决策

| 决策 | 结论 |
|------|------|
| 3D 库 | Three.js MMDLoader（已内置） |
| MMD 物理 | 不启用，桌宠不需要头发/裙子物理摆动 |
| 动画策略 | VMD idle + 程序化微动 + 呼吸混合 |
| 贴图管理 | 保持 PMX 原始目录结构，复制时保留 tex/ 路径 |
| 模型导入 | 用户选 .pmx 文件 → 后端自动复制贴图 + 写索引 |
| 相机控制 | 自定义实现（拖拽旋转 + 滚轮缩放） |

---

## 八、测试模型

使用 D:\文档\ai女友 下已有模型：

```
测试 1: 凯茜娅-狂诗 (PMX)
  - 验证贴图加载正确（相对路径 tex/）
  - 验证骨骼动画（如有 VMD）
  - 验证呼吸效果 + 程序化微动

测试 2: 凯茜娅-朝翼 (PMX)
  - 验证不同贴图目录结构的兼容性

测试 3: 沐浪倦姿 (PMX)
  - 验证模型切换的无泄漏
```

---

**版本**: 1.0
**日期**: 2026-06-08
