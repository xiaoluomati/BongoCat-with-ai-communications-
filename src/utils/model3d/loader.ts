import * as THREE from 'three'
import { MMDLoader } from 'three/examples/jsm/loaders/MMDLoader.js'

export class ModelLoader {
  private loader: MMDLoader

  constructor() {
    this.loader = new MMDLoader()
  }

  async loadModel(
    modelPath: string,
    onProgress?: (pct: number) => void,
  ): Promise<THREE.SkinnedMesh> {
    return this.loader.loadAsync(
      modelPath,
      null,  // motion VMD — loaded separately
      null,  // camera VMD — not needed
      (e) => {
        if (e.lengthComputable && onProgress) {
          onProgress(Math.round((e.loaded / e.total) * 100))
        }
      },
    )
  }

  async loadMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    return this.loader.loadVMDAsync(vmdPath)
  }
}
