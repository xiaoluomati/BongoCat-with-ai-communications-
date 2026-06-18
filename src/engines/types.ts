export interface ModelSize {
  width: number
  height: number
}

export interface DisplayEngine {
  /** Load a model and return its native size */
  load(path: string): Promise<ModelSize>

  /** Destroy the engine and release all resources */
  destroy(): void

  /** Handle window resize */
  resize(width: number, height: number): void

  // ── Interaction (optional, engine-specific) ──

  /** Mouse moved over the canvas (L2D: eye tracking, 3D: no-op) */
  handleMouseMove?(x: number, y: number): void

  /** Keyboard key pressed/released (L2D: gesture change) */
  handleKeyChange?(isLeft: boolean, pressed: boolean): void

  /** Drag rotation delta (3D only) */
  handleRotate?(dx: number): void

  /** Scroll zoom delta (3D only) */
  handleZoom?(delta: number): void

  /** Play a named expression/morph */
  playExpression?(name: string): void
}
