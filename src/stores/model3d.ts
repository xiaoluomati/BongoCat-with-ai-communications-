import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface Model3DInfo {
  id: string
  name: string
  pmxPath: string
  motions: Record<string, string>
  addedAt: string
}

export const useModel3DStore = defineStore('model3d', () => {
  const models = ref<Model3DInfo[]>([])
  const currentModelId = ref<string | null>(null)
  const breatheEnabled = ref(true)
  const proceduralEnabled = ref(true)

  const currentModel = computed(() =>
    models.value.find(m => m.id === currentModelId.value) ?? null,
  )

  const currentPmxPath = computed(() => currentModel.value?.pmxPath ?? null)
  const currentMotions = computed(() => currentModel.value?.motions ?? {})

  function setModels(list: Model3DInfo[]) {
    models.value = list
  }

  function selectModel(id: string) {
    currentModelId.value = id
  }

  return {
    models, currentModelId, breatheEnabled, proceduralEnabled,
    currentModel, currentPmxPath, currentMotions,
    setModels, selectModel,
  }
})
