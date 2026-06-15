# 3D 模型桌宠扩展 — 技术调研

> **日期**: 2026-06-08
> **目的**: 评估在现有 BongoCat 项目中新增 3D 模型展示页面的可行性和复杂度

---

## 一、现有架构回顾

当前项目使用 **PixiJS** 渲染 Live2D Cubism4 2D 模型：

```
主窗口 (main/index.vue)
  └─ <canvas id="live2dCanvas">
       └─ PixiJS Application (WebGL)
            └─ Live2DModel (pixi-live2d-display + Cubism4 SDK)
```

已有能力：
- ✅ WebGL 渲染管线（PixiJS 6.x）
- ✅ Canvas 元素绑定
- ✅ 鼠标交互（拖拽、右键菜单）
- ✅ 窗口置顶、无边框
- ✅ Live2D 参数控制（呼吸、手部动作参数已内置）

---

## 二、技术方案对比

### 方案 A: Three.js + GLTF/GLB 模型

| 维度 | 评价 |
|------|------|
| 生态成熟度 | ★★★★★ 最成熟的 Web 3D 库 |
| 模型资源 | ★★★★★ GLTF/GLB 格式通用，Sketchfab/VRoid 等大量可用 |
| 学习曲线 | ★★★☆☆ 中等，文档完善 |
| 包大小 | 约 150KB gzipped（three.js core） |
| 与 PixiJS 共存 | 需要独立 canvas 或与 PixiJS 共享 WebGL 上下文 |
| 性能 | WebGL2 渲染，与当前 Live2D 类似 |

### 方案 B: Babylon.js

| 维度 | 评价 |
|------|------|
| 生态成熟度 | ★★★★☆ 成熟但社区小于 Three.js |
| 模型资源 | ★★★★☆ 同样支持 GLTF/GLB |
| 学习曲线 | ★★★★☆ 更完整，开箱即用 |
| 包大小 | 约 300KB gzipped（core） |
| 适合场景 | 需要 PBR、阴影、后处理时优势明显，桌宠场景过度 |

### 方案 C: 复用 PixiJS + 伪 3D

| 维度 | 评价 |
|------|------|
| 实现方式 | 用 2D 精灵图（spritesheet）模拟 3D 旋转、透视 |
| 复杂度 | 低，不需要 3D 库 |
| 效果上限 | 有限，无法实现真正的 3D 交互 |
| 适用性 | 仅适合简单场景，不推荐 |

### 推荐: **方案 A — Three.js**

原因：
- 模型资源最丰富（VRM、MMD、GLTF 都有成熟加载器）
- 社区最大，遇到问题容易解决
- 与 PixiJS 可共存（各自独立 canvas，切换页面时销毁/初始化）
- 桌面宠物不需要 Babylon.js 的重量级功能

---

## 三、"呼吸"与"自然运动"实现分析

### 3.1 呼吸效果

**难度: 极低（< 10 行代码）**

正弦波驱动模型 scale 或 morph target：

```javascript
// 在 render loop 中
const breathe = 1 + Math.sin(Date.now() * 0.001) * 0.02  // ±2% 缩放
model.scale.setScalar(breathe)
```

或使用模型的 blend shape / morph target（如果模型自带呼吸 blendshape，效果更自然）。

### 3.2 自然运动（Idle Animation）

**难度: 低-中（取决于实现方式）**

**方式 1 — 烘焙动画（推荐）**

如果 3D 模型自带 idle 动画（GLTF 的 animation clip），直接播放：

```javascript
const mixer = new THREE.AnimationMixer(model)
const idleAction = mixer.clipAction(gltf.animations[0])
idleAction.play()

// render loop
mixer.update(deltaTime)
```

大部分 VRoid / MMD / Sketchfab 模型都带 idle 动画。

**方式 2 — 程序化动画（无烘焙动画时）**

用 Perlin/Simplex noise 驱动骨骼微旋转，模拟"站着不动的微小晃动"：

```javascript
// 简易实现：低频 noise 驱动模型整体微摆 + 头部微动
const noise = createNoise3D()
model.rotation.z = noise(Date.now() * 0.0005, 0, 0) * 0.05
headBone.rotation.x = noise(0, Date.now() * 0.0008, 0) * 0.1
```

**方式 3 — 混合方案（最佳）**

- 有 idle 动画 → 循环播放
- 每隔 5-15 秒随机触发一个 gesture 动画（眨眼、摆头、挥手）
- 呼吸效果叠加在所有动画上

---

## 四、模型格式选择

| 格式 | 来源 | 文件大小 | 动画支持 | 推荐度 |
|------|------|---------|---------|--------|
| **VRM 1.0** | VRoid Studio 等 | 5-20MB | ✅ 内置 | ★★★★★ |
| **GLTF/GLB** | Sketchfab, Blender | 3-15MB | ✅ | ★★★★★ |
| **MMD (PMX)** | MikuMikuDance | 5-30MB | ✅ | ★★★☆☆ |
| **FBX** | 各类建模软件 | 较大 | ✅ | ★★★☆☆ |

**推荐 VRM 或 GLB**：VRM 专门为角色设计，自带表情 blendshape、视线追踪；GLB 最通用。

---

## 五、与现有代码集成方案

### 5.1 窗口架构

新增一个独立 Tauri 窗口，与现有 Live2D 主窗口并存：

```
tauri.conf.json
  windows:
    - main          (Live2D, 已有)
    - preference    (设置, 已有)
    - chat          (聊天, 已有)
    - comprehensive_function (功能面板, 已有)
    - model3d       (3D 模型, 新增)        ← 新窗口
```

或复用主窗口，通过切换标签切换 2D/3D 展示。

### 5.2 页面组件

```
src/pages/model3d/index.vue       ← 3D 展示页
  ├─ Three.js 初始化
  ├─ 模型加载 (GLTFLoader / VRMLoader)
  ├─ 呼吸动画 (sine wave)
  ├─ 自然运动 (idle animation + procedural)
  ├─ 鼠标交互 (旋转/拖拽)
  └─ 右键菜单 (与现有共享 useSharedMenu)
```

### 5.3 渲染隔离

PixiJS 和 Three.js 不能共享同一个 WebGL context。两个方案：

- **方案 A**: 不同窗口各用各的 canvas（简单，内存增量约 50MB）
- **方案 B**: 同一窗口动态切换（需要销毁 PixiJS → 初始化 Three.js，有切换延迟）

**推荐方案 A** — 独立窗口，简单可靠。

---

## 六、性能与内存评估

### 6.1 内存

当前 Live2D 主窗口内存：~80-180MB（PixiJS + Live2D 模型）

| 新增项 | 预估增量 |
|--------|---------|
| Three.js + 场景 | +20-30MB |
| 3D 模型（VRM/GLB 5-20MB） | +10-40MB（含纹理） |
| 新 WebView 窗口 | +30-50MB |
| **总计增量** | **~60-120MB** |
| 应用总内存（6 窗口） | ~260-420MB |

### 6.2 帧率

Three.js 在桌面 WebView2 上对简单场景（单模型 + 无阴影 + 无后处理）可达 60fps。与 Live2D 窗口并存时各自独立渲染，互不影响。

---

## 七、关键依赖

```json
{
  "three": "^0.170.0",          // Three.js 核心
  "@pixiv/three-vrm": "^3.0.0"  // VRM 模型加载（如使用 VRM 格式）
}
// GLTF/GLB 格式不需要额外依赖，Three.js 内置 GLTFLoader
```

额外包增量约 **160KB gzipped**（three.js core + VRM loader）。

---

## 八、开发工作量估算

| 任务 | 复杂度 | 预估时间 |
|------|--------|---------|
| Three.js 初始化 + Canvas 绑定 | 低 | 0.5 天 |
| 模型加载 (GLTF/VRM) | 低 | 0.5 天 |
| 呼吸效果 (sine wave) | 极低 | 0.5h |
| 自然运动 (idle animation + procedural) | 中 | 1-2 天 |
| 鼠标交互 (旋转/缩放/拖拽) | 低 | 0.5 天 |
| 新窗口配置 + 路由 | 低 | 0.5 天 |
| 模型管理 UI (上传/切换/删除) | 中 | 1-2 天 |
| 配置持久化 | 低 | 0.5 天 |
| 与现有功能联动 (表情触发等) | 中 | 1 天 |
| **合计** | | **5-8 天** |

---

## 九、结论

**完全可行，复杂度在可控范围内。**

- 呼吸效果 ≈ 一行正弦波，几乎零成本
- 自然运动 ≈ 播放模型自带 idle 动画 + 少量 procedural noise，1-2 天
- 最大工作量在模型管理 UI 和与现有系统的联调
- 内存增量 ~60-120MB，可接受
- 推荐 Three.js + 独立窗口方案

### 建议分两阶段：

**第一阶段（MVP，3 天）**: 加载模型 → 呼吸效果 → 基础 idle 动画 → 鼠标旋转

**第二阶段（增强，3-5 天）**: 模型管理 UI → 随机 gesture → 微表情 → 与聊天联动

---

**版本**: 1.0
**更新**: 2026-06-08
