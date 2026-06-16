import * as THREE from 'three'

export interface AnimationState {
  mixer: THREE.AnimationMixer | null
  currentAction: THREE.AnimationAction | null
  actionMap: Map<string, THREE.AnimationClip>

  breatheEnabled: boolean
  breatheIntensity: number
  breathePeriod: number

  proceduralEnabled: boolean
}

export function createAnimationState(mixer: THREE.AnimationMixer | null): AnimationState {
  return {
    mixer,
    currentAction: null,
    actionMap: new Map(),
    breatheEnabled: true,
    breatheIntensity: 0.02,
    breathePeriod: 3.0,
    proceduralEnabled: true,
  }
}

// ── 呼吸效果 ──
function applyBreathe(model: THREE.Object3D, state: AnimationState, time: number) {
  if (!state.breatheEnabled) return
  const breathe = 1 + Math.sin(time * ((2 * Math.PI) / state.breathePeriod)) * state.breatheIntensity
  model.scale.setScalar(breathe)
}

// ── 程序化微动 ──
function applyProceduralMotion(model: THREE.Object3D, time: number) {
  model.rotation.z = Math.sin(time * 0.7) * 0.03
  model.rotation.y = Math.sin(time * 0.5 + 1) * 0.02
  model.position.y += Math.sin(time * 0.6) * 0.003
}

// ── 切换骨骼动作 ──
export function playMotion(state: AnimationState, name: string, fadeIn = 0.5) {
  if (!state.mixer) return
  const clip = state.actionMap.get(name)
  if (!clip) return

  const newAction = state.mixer.clipAction(clip)
  if (state.currentAction) {
    newAction.crossFadeFrom(state.currentAction, fadeIn, true)
  }
  newAction.play()
  state.currentAction = newAction
}

// ── Render loop ──
export function startRenderLoop(
  state: AnimationState,
  model: THREE.Object3D,
  renderer: THREE.WebGLRenderer,
  scene: THREE.Scene,
  camera: THREE.Camera,
) {
  const clock = new THREE.Clock()

  function tick() {
    const delta = clock.getDelta()
    const elapsed = clock.getElapsedTime()

    state.mixer?.update(delta)
    applyBreathe(model, state, elapsed)

    const hasActiveMotion = state.currentAction?.isRunning()
    if (state.proceduralEnabled && !hasActiveMotion) {
      applyProceduralMotion(model, elapsed)
    }

    renderer.render(scene, camera)
    requestAnimationFrame(tick)
  }

  requestAnimationFrame(tick)
}
