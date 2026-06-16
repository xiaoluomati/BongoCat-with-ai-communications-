import { ref, type Ref } from 'vue'
import * as THREE from 'three'
import { createScene, resizeScene, type SceneContext } from '@/utils/model3d/scene'
import { ModelLoader } from '@/utils/model3d/loader'
import { createAnimationState, playMotion, startRenderLoop, type AnimationState } from '@/utils/model3d/animation'
import { ModelInteraction } from '@/utils/model3d/interaction'
import { useModel3DStore } from '@/stores/model3d'

export type Status = 'empty' | 'loading' | 'ready' | 'error'

export function useModel3D() {
  const store = useModel3DStore()
  const status: Ref<Status> = ref('empty')
  const progress = ref(0)
  const error = ref<string | null>(null)

  let ctx: SceneContext | null = null
  let model: THREE.SkinnedMesh | null = null
  let anim: AnimationState | null = null
  let interaction: ModelInteraction | null = null
  const loader = new ModelLoader()

  async function init(canvas: HTMLCanvasElement) {
    ctx = createScene(canvas)
    window.addEventListener('resize', onResize)
  }

  function onResize() {
    if (!ctx) return
    const canvas = ctx.renderer.domElement
    resizeScene(ctx, canvas.clientWidth, canvas.clientHeight)
  }

  async function loadCurrentModel() {
    const path = store.currentPmxPath
    if (!path || !ctx) return

    status.value = 'loading'
    progress.value = 0
    error.value = null

    try {
      unloadModel()

      model = await loader.loadModel(path, (pct) => {
        progress.value = pct
      })
      ctx.scene.add(model)

      const mixer = new THREE.AnimationMixer(model)
      anim = createAnimationState(mixer)
      anim.breatheEnabled = store.breatheEnabled
      anim.proceduralEnabled = store.proceduralEnabled

      // Load VMD motions
      const motions = store.currentMotions
      for (const [name, vmdPath] of Object.entries(motions)) {
        try {
          const clip = await loader.loadMotion(vmdPath)
          anim.actionMap.set(name, clip)
          if (name === 'idle') playMotion(anim, 'idle')
        } catch {
          console.warn(`[model3d] failed to load motion: ${name}`)
        }
      }

      startRenderLoop(anim, model, ctx.renderer, ctx.scene, ctx.camera)

      interaction?.destroy()
      interaction = new ModelInteraction(model, ctx.camera, ctx.renderer.domElement)

      status.value = 'ready'
    } catch (e) {
      error.value = String(e)
      status.value = 'error'
    }
  }

  function unloadModel() {
    if (model) {
      ctx?.scene.remove(model)
      model = null
    }
    anim?.mixer?.stopAllAction()
    anim = null
    interaction?.destroy()
    interaction = null
  }

  function destroy() {
    unloadModel()
    ctx?.renderer.dispose()
    ctx = null
    window.removeEventListener('resize', onResize)
  }

  return { status, progress, error, init, loadCurrentModel, destroy }
}
