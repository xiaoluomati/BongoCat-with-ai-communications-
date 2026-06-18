import type { DisplayEngine, ModelSize } from './types'
import * as THREE from 'three'
import { convertFileSrc } from '@tauri-apps/api/core'
import { createScene, resizeScene, type SceneContext } from '@/utils/model3d/scene'
import { ModelLoader } from '@/utils/model3d/loader'
import { createAnimationState, playMotion, startRenderLoop, type AnimationState } from '@/utils/model3d/animation'
import { ModelInteraction } from '@/utils/model3d/interaction'
import { useModel3DStore } from '@/stores/model3d'

export class ThreeEngine implements DisplayEngine {
  private ctx: SceneContext | null = null
  private model: THREE.SkinnedMesh | null = null
  private anim: AnimationState | null = null
  private interaction: ModelInteraction | null = null
  private stopLoop: (() => void) | null = null
  private resizeObserver: ResizeObserver | null = null
  private loader = new ModelLoader()
  private store = useModel3DStore()

  async load(path: string): Promise<ModelSize> {
    this.destroy()

    // Create canvas if needed, or reuse existing
    const canvas = document.getElementById('live2dCanvas') as HTMLCanvasElement
    if (!canvas) throw new Error('Canvas not found')

    this.ctx = createScene(canvas)
    this.resizeObserver = new ResizeObserver(() => {
      if (!this.ctx) return
      resizeScene(this.ctx, canvas.clientWidth, canvas.clientHeight)
    })
    this.resizeObserver.observe(canvas.parentElement || canvas)

    // Load model
    this.model = await this.loader.loadModel(path)
    this.ctx.scene.add(this.model)

    const mixer = new THREE.AnimationMixer(this.model)
    this.anim = createAnimationState(mixer)
    this.anim.breatheEnabled = this.store.breatheEnabled
    this.anim.proceduralEnabled = this.store.proceduralEnabled

    // Load VMD motions
    const motions = this.store.currentMotions
    for (const [name, vmdPath] of Object.entries(motions)) {
      try {
        const clip = await this.loader.loadMotion(vmdPath)
        this.anim.actionMap.set(name, clip)
        if (name === 'idle') playMotion(this.anim, 'idle')
      } catch { /* skip */ }
    }

    this.stopLoop = startRenderLoop(this.anim, this.model, this.ctx.renderer, this.ctx.scene, this.ctx.camera)
    this.interaction = new ModelInteraction(this.model, this.ctx.camera, canvas)

    return { width: canvas.clientWidth, height: canvas.clientHeight }
  }

  destroy(): void {
    this.stopLoop?.()
    this.stopLoop = null
    this.interaction?.destroy()
    this.interaction = null
    if (this.model) {
      this.ctx?.scene.remove(this.model)
      this.model = null
    }
    this.anim?.mixer?.stopAllAction()
    this.anim = null
    this.resizeObserver?.disconnect()
    this.resizeObserver = null
    this.ctx?.renderer.dispose()
    this.ctx = null
    this.loader.destroy()
  }

  resize(width: number, height: number): void {
    if (!this.ctx) return
    resizeScene(this.ctx, width, height)
  }

  handleRotate(dx: number): void {
    if (!this.model) return
    this.model.rotation.y += dx * 0.01
  }

  handleZoom(delta: number): void {
    if (!this.ctx) return
    const cam = this.ctx.camera
    cam.position.z = Math.max(10, Math.min(60, cam.position.z + delta * 0.02))
  }
}
