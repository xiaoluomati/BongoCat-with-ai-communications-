import type { DisplayEngine, ModelSize } from './types'
import live2d from '@/utils/live2d'
import { useCatStore } from '@/stores/cat'
import { useModelStore } from '@/stores/model'

export class Live2DEngine implements DisplayEngine {
  private catStore = useCatStore()
  private modelStore = useModelStore()

  async load(path: string): Promise<ModelSize> {
    const result = await live2d.load(path)
    return { width: result.width, height: result.height }
  }

  destroy(): void {
    live2d.destroy()
  }

  resize(width: number, height: number): void {
    // Live2D resize is handled by the model store's scale logic
    // This is a no-op — the engine doesn't track window dimensions,
    // the caller (main/index.vue) handles it via setSize
  }

  handleMouseMove(x: number, y: number): void {
    // Implemented externally via useModel().handleMouseMove
  }

  handleKeyChange(isLeft: boolean, pressed: boolean): void {
    const id = isLeft ? 'CatParamLeftHandDown' : 'CatParamRightHandDown'
    live2d.setParameterValue(id, pressed)
  }

  playExpression(name: string): void {
    live2d.playExpressions(name)
  }
}
