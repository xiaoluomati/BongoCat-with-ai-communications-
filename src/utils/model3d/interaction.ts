import * as THREE from 'three'

export class ModelInteraction {
  private isDragging = false
  private prevX = 0
  private rotateY = 0
  private cameraDistance: number

  constructor(
    private model: THREE.Object3D,
    private camera: THREE.PerspectiveCamera,
    private canvas: HTMLCanvasElement,
  ) {
    this.cameraDistance = camera.position.z
    canvas.addEventListener('mousedown', this.onMouseDown)
    canvas.addEventListener('mousemove', this.onMouseMove)
    window.addEventListener('mouseup', this.onMouseUp)
    canvas.addEventListener('wheel', this.onWheel, { passive: false })
  }

  private onMouseDown = (e: MouseEvent) => {
    if (e.button === 0) {
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
    this.cameraDistance += e.deltaY * 0.02
    this.cameraDistance = Math.max(10, Math.min(60, this.cameraDistance))
    this.camera.position.z = this.cameraDistance
  }

  destroy() {
    this.canvas.removeEventListener('mousedown', this.onMouseDown)
    this.canvas.removeEventListener('mousemove', this.onMouseMove)
    window.removeEventListener('mouseup', this.onMouseUp)
    this.canvas.removeEventListener('wheel', this.onWheel)
  }
}
