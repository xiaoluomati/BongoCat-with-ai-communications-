import * as THREE from 'three'
import { MMDLoader } from 'three/examples/jsm/loaders/MMDLoader.js'
import { convertFileSrc } from '@tauri-apps/api/core'
import { readFile, readDir } from '@tauri-apps/plugin-fs'

export class ModelLoader {
  private loader: MMDLoader
  private textureCache: Map<string, string> = new Map()

  constructor() {
    this.loader = new MMDLoader()
  }

  async loadModel(
    modelPath: string,
    onProgress?: (pct: number) => void,
  ): Promise<THREE.SkinnedMesh> {
    const url = convertFileSrc(modelPath)
    console.log('[loader] loading PMX via asset protocol:', url)

    // Pre-load all textures from the model directory into blob URLs
    const baseDir = modelPath.replace(/[/\\][^/\\]*$/, '')
    await this.preloadTextures(baseDir)

    return new Promise((resolve, reject) => {
      this.loader.load(
        url,
        (model) => {
          console.log('[loader] PMX loaded, fixing textures...')
          this.fixTextures(model, baseDir)
          resolve(model as THREE.SkinnedMesh)
        },
        (e) => {
          if (e.lengthComputable && onProgress) {
            onProgress(Math.round((e.loaded / e.total) * 100))
          }
        },
        (e) => reject(new Error(e instanceof ErrorEvent ? e.message : String(e))),
      )
    })
  }

  // Pre-load all image files from the model directory
  private async preloadTextures(baseDir: string) {
    try {
      const entries = await readDir(baseDir)
      await this.scanDir(entries, baseDir)
      console.log(`[loader] preloaded ${this.textureCache.size} textures`)
    } catch (e) {
      console.warn('[loader] texture preload failed:', e)
    }
  }

  private async scanDir(entries: any[], parentPath: string) {
    for (const entry of entries) {
      const fullPath = parentPath + '\\' + entry.name
      if (entry.isDirectory) {
        try {
          const sub = await readDir(fullPath)
          await this.scanDir(sub, fullPath)
        } catch { /* skip */ }
      } else {
        const lower = entry.name.toLowerCase()
        if (lower.endsWith('.png') || lower.endsWith('.jpg') || lower.endsWith('.jpeg')
          || lower.endsWith('.bmp') || lower.endsWith('.tga') || lower.endsWith('.dds')) {
          try {
            const bytes = await readFile(fullPath)
            const blob = new Blob([bytes])
            const blobUrl = URL.createObjectURL(blob)
            this.textureCache.set(entry.name.toLowerCase(), blobUrl)
          } catch { /* skip */ }
        }
      }
    }
  }

  // Replace failed textures with pre-loaded blob URLs
  private fixTextures(model: THREE.Object3D, baseDir: string) {
    model.traverse((obj) => {
      const mesh = obj as THREE.Mesh
      if (!mesh.material) return
      const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material]
      for (const mat of materials) {
        this.fixMaterial(mat as THREE.MeshPhongMaterial)
      }
    })
  }

  private fixMaterial(mat: THREE.MeshPhongMaterial) {
    const textureKeys = ['map', 'gradientMap', 'alphaMap', 'specularMap', 'envMap'] as const
    for (const key of textureKeys) {
      const tex = (mat as any)[key] as THREE.Texture | null
      if (!tex || !tex.image) continue

      // Try to find a matching blob URL from the cache
      // Extract filename from what MMDLoader tried to load
      const src = tex.image instanceof HTMLImageElement ? tex.image.src : ''
      if (!src) continue

      // Try to match by filename
      for (const [name, blobUrl] of this.textureCache) {
        if (src.includes(encodeURIComponent(name)) || src.includes(name) || src.endsWith('/' + name)) {
          console.log('[loader] replacing texture:', name)
          const newTex = new THREE.TextureLoader().load(blobUrl)
          newTex.wrapS = tex.wrapS
          newTex.wrapT = tex.wrapT
          newTex.flipY = tex.flipY
          ;(mat as any)[key] = newTex
          break
        }
      }
    }
  }

  async loadMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    const url = convertFileSrc(vmdPath)
    return this.loader.loadVMDAsync(url)
  }
}
