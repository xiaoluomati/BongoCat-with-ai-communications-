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
  // MMD models have origin at feet; look at upper body (Y=10 in MMD units)
  camera.position.set(0, 10, 20)
  camera.lookAt(0, 10, 0)

  // Match renderer size to canvas
  renderer.setSize(canvas.clientWidth, canvas.clientHeight)

  const ambient = new THREE.AmbientLight(0xffffff, 0.8)
  scene.add(ambient)

  const directional = new THREE.DirectionalLight(0xffffff, 0.6)
  directional.position.set(1, 2, 3)
  scene.add(directional)

  return { renderer, scene, camera }
}

export function resizeScene(ctx: SceneContext, width: number, height: number) {
  ctx.renderer.setSize(width, height)
  ctx.camera.aspect = width / height
  ctx.camera.updateProjectionMatrix()
}
