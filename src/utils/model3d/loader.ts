import * as THREE from 'three'
import { MMDLoader } from 'three/examples/jsm/loaders/MMDLoader.js'
import { convertFileSrc } from '@tauri-apps/api/core'
import { readFile, readDir } from '@tauri-apps/plugin-fs'

export class ModelLoader {
  private pmxLoader: MMDLoader
  private vmdLoader: MMDLoader
  private textureBlobs: Map<string, string> = new Map()

  constructor() {
    // Create a LoadingManager that intercepts texture requests
    const manager = new THREE.LoadingManager()
    manager.setURLModifier((url) => {
      // Check if we have a preloaded blob URL for this texture
      const fileName = decodeURIComponent(url.split('/').pop() || url.split('\\').pop() || '')
      const lower = fileName.toLowerCase()
      const match = this.textureBlobs.get(lower)
      if (match) {
        console.log('[loader] texture resolved via blob:', lower)
        return match
      }
      // Try matching with encoding variations
      for (const [key, blobUrl] of this.textureBlobs) {
        if (url.includes(key) || url.toLowerCase().endsWith('/' + key.toLowerCase())) {
          console.log('[loader] texture fuzzy matched:', key)
          return blobUrl
        }
      }
      // Fall back to original URL
      return url
    })

    this.pmxLoader = new MMDLoader(manager)
    this.vmdLoader = new MMDLoader()
  }

  async loadModel(
    modelPath: string,
    onProgress?: (pct: number) => void,
  ): Promise<THREE.SkinnedMesh> {
    const url = convertFileSrc(modelPath)
    console.log('[loader] PMX asset URL:', url)

    // Pre-load all textures from the model directory
    const baseDir = modelPath.replace(/[/\\][^/\\]*$/, '')
    await this.preloadTextures(baseDir)

    return new Promise((resolve, reject) => {
      this.pmxLoader.load(
        url,
        (model) => resolve(model as THREE.SkinnedMesh),
        (e) => {
          if (e.lengthComputable && onProgress) {
            onProgress(Math.round((e.loaded / e.total) * 100))
          }
        },
        (e) => reject(new Error(e instanceof ErrorEvent ? e.message : String(e))),
      )
    })
  }

  private async preloadTextures(baseDir: string) {
    // Revoke old blob URLs before clearing to free memory
    for (const url of this.textureBlobs.values()) {
      URL.revokeObjectURL(url)
    }
    this.textureBlobs.clear()
    await this.scanDir(baseDir)
    console.log(`[loader] preloaded ${this.textureBlobs.size} textures`)
  }

  // Clean up all blob URLs and release memory
  destroy() {
    for (const url of this.textureBlobs.values()) {
      URL.revokeObjectURL(url)
    }
    this.textureBlobs.clear()
  }

  private async scanDir(dirPath: string) {
    try {
      const entries = await readDir(dirPath)
      for (const entry of entries) {
        const fullPath = dirPath + '\\' + entry.name
        if (entry.isDirectory) {
          await this.scanDir(fullPath)
        } else {
          const lower = entry.name.toLowerCase()
          if (lower.endsWith('.png') || lower.endsWith('.jpg') || lower.endsWith('.jpeg')
            || lower.endsWith('.bmp') || lower.endsWith('.tga') || lower.endsWith('.dds')) {
            try {
              const bytes = await readFile(fullPath)
              const blob = new Blob([bytes])
              const blobUrl = URL.createObjectURL(blob)
              this.textureBlobs.set(lower, blobUrl)
            } catch { /* skip */ }
          }
        }
      }
    } catch (e) {
      console.warn('[loader] scanDir error:', e)
    }
  }

  async loadMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    const url = convertFileSrc(vmdPath)
    return this.vmdLoader.loadVMDAsync(url)
  }
}
