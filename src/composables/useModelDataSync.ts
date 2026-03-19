import { emit } from '@tauri-apps/api/event'
import { onMounted, ref } from 'vue'

import { useTauriListen } from './useTauriListen'

import { LISTEN_KEY } from '@/constants'

interface Expression {
  Name: string
  File: string
  Description?: string
}

interface Model {
  id: string
  path: string
  mode: string
  isPreset: boolean
}

export function useModelDataSync() {
  const currentModel = ref<Model | null>(null)
  const expressions = ref<Expression[]>([])
  const isModelDataLoaded = ref(false)

  // 监听来自主窗口的模型数据同步
  useTauriListen(LISTEN_KEY.MODEL_DATA_SYNC, (event: any) => {
    const { currentModel: model, expressions: exprs } = event.payload
    currentModel.value = model
    expressions.value = exprs || []
    isModelDataLoaded.value = true
  })

  // 请求主窗口同步模型数据
  const requestModelData = async () => {
    await emit(LISTEN_KEY.REQUEST_MODEL_DATA, {})
  }

  onMounted(() => {
    requestModelData()
  })

  return {
    currentModel,
    expressions,
    isModelDataLoaded,
    requestModelData,
  }
}
