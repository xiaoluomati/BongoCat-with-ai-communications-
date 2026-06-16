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

      const src = tex.image instanceof HTMLImageElement ? tex.image.src : ''
      if (!src) continue

      console.log('[loader] texture src:', key, src.substring(0, 100))
      let matched = false

      for (const [name, blobUrl] of this.textureCache) {
        const encoded = encodeURIComponent(name)
        if (src.includes(encoded) || src.includes(name) || src.toLowerCase().endsWith('/' + name.toLowerCase())) {
          console.log('[loader] MATCHED, replacing with blob:', name)
          const newTex = new THREE.TextureLoader().load(blobUrl)
          newTex.wrapS = tex.wrapS
          newTex.wrapT = tex.wrapT
          newTex.flipY = tex.flipY
          ;(mat as any)[key] = newTex
          matched = true
          break
        }
      }
      if (!matched) {
        console.warn('[loader] NO MATCH for:', src.substring(0, 100))
        console.log('[loader] cache keys:', [...this.textureCache.keys()].join(', '))
      }
    }
  }

  async loadMotion(vmdPath: string): Promise<THREE.AnimationClip> {
    const url = convertFileSrc(vmdPath)
    return this.loader.loadVMDAsync(url)
  }
}
