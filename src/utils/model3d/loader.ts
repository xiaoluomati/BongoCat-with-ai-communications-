import * as THREE from 'three'
import { MMDLoader } from 'three/examples/jsm/loaders/MMDLoader.js'
import { convertFileSrc } from '@tauri-apps/api/core'

export class ModelLoader {
  private loader: MMDLoader

  constructor() {
    this.loader = new MMDLoader()
  }

  async loadModel(
    modelPath: string,
    onProgress?: (pct: number) => void,
  ): Promise<THREE.SkinnedMesh> {
    // Tauri webview requires asset protocol URLs to access local files
    const url = convertFileSrc(modelPath)
    return this.loader.loadAsync(
      url,
      null,
      null,
      (e) => {
        if (e.lengthComputable && onProgress) {
          onProgress(Math.round((e.loaded / e.total) * 100))
        }
      },
    )
  }

  async loadMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    const url = convertFileSrc(vmdPath)
    return this.loader.loadVMDAsync(url)
  }
}
