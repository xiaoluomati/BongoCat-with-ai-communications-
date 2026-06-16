import * as THREE from 'three'

export interface SceneContext {
  renderer: THREE.WebGLRenderer
  scene: THREE.Scene
  camera: THREE.PerspectiveCamera
}

export function createScene(canvas: HTMLCanvasElement): SceneContext {
  const renderer = new THREE.WebGLRenderer({
    canvas,
    alpha: true,
    antialias: true,
  })
  renderer.setPixelRatio(window.devicePixelRatio)

  const scene = new THREE.Scene()

  const camera = new THREE.PerspectiveCamera(
    45,
    canvas.clientWidth / canvas.clientHeight,
    0.1,
    100,
  )
  // MMD models have origin at feet (~10 MMD units = ~1.25m height)
  // Position camera further back and higher to see full body
  camera.position.set(0, 10, 45)
  camera.lookAt(0, 11, 0)

  // Match renderer size to canvas physical size
  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))

  const ambient = new THREE.AmbientLight(0xffffff, 0.8)
  scene.add(ambient)

  const directional = new THREE.DirectionalLight(0xffffff, 0.6)
  directional.position.set(1, 2, 3)
  scene.add(directional)

  return { renderer, scene, camera }
}

export function resizeScene(ctx: SceneContext, width: number, height: number) {
  if (width === 0 || height === 0) return
  ctx.renderer.setSize(width, height, false)
  ctx.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  ctx.camera.aspect = width / height
  ctx.camera.updateProjectionMatrix()
}
