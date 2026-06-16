<script setup lang="ts">
import { DeleteOutlined, PlusOutlined, PlayCircleOutlined } from '@ant-design/icons-vue'
import { Button, Card, List, Modal, message, Spin } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ref, onMounted, computed } from 'vue'
import { useModel3DStore, type Model3DInfo } from '@/stores/model3d'

const store = useModel3DStore()
const loading = ref(false)
const importName = ref('')

const models = computed(() => store.models)
const currentId = computed(() => store.currentModelId)

onMounted(async () => {
  await loadModels()
})

async function loadModels() {
  loading.value = true
  try {
    const list = await invoke<Model3DInfo[]>('list_3d_models')
    store.setModels(list)
    // Auto-select first model if none selected
    if (!store.currentModelId && list.length > 0) {
      store.selectModel(list[0].id)
    }
  } catch (e) {
    message.error('加载模型列表失败: ' + String(e))
  } finally {
    loading.value = false
  }
}

async function handleImport() {
  const file = await open({
    filters: [{ name: 'PMX Model', extensions: ['pmx'] }],
    multiple: false,
  })
  if (!file) return

  const name = (file as string).split(/[/\\]/).pop()?.replace('.pmx', '') || '未命名'
  loading.value = true
  try {
    await invoke<Model3DInfo>('add_3d_model', { name, sourcePath: file })
    message.success('模型导入成功')
    await loadModels()
  } catch (e) {
    message.error('导入失败: ' + String(e))
  } finally {
    loading.value = false
  }
}

async function handleDelete(model: Model3DInfo) {
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除 "${model.name}" 吗？`,
    okText: '删除',
    cancelText: '取消',
    okType: 'danger',
    onOk: async () => {
      try {
        await invoke('remove_3d_model', { id: model.id })
        if (currentId.value === model.id) store.selectModel('')
        message.success('已删除')
        await loadModels()
      } catch (e) {
        message.error('删除失败: ' + String(e))
      }
    },
  })
}

function handleSelect(model: Model3DInfo) {
  store.selectModel(model.id)
  message.success(`已选择: ${model.name}`)
}

async function handleAddMotion(model: Model3DInfo) {
  const file = await open({
    filters: [{ name: 'VMD Motion', extensions: ['vmd'] }],
    multiple: false,
  })
  if (!file) return

  const name = (file as string).split(/[/\\]/).pop()?.replace('.vmd', '') || 'motion'
  try {
    await invoke('add_model_motion', { modelId: model.id, motionName: name, vmdPath: file })
    message.success(`动作 "${name}" 已添加`)
    await loadModels()
  } catch (e) {
    message.error('添加动作失败: ' + String(e))
  }
}
</script>

<template>
  <div class="model3d-config">
    <Spin :spinning="loading">
      <Card title="3D 模型管理" size="small">
        <template #extra>
          <Button type="primary" size="small" @click="handleImport">
            <PlusOutlined /> 导入 PMX 模型
          </Button>
        </template>

        <List
          v-if="models.length > 0"
          :data-source="models"
          size="small"
        >
          <template #renderItem="{ item }">
            <List.Item>
              <div class="model-row">
                <div class="model-info">
                  <span class="model-name">{{ item.name }}</span>
                  <span
                    v-if="item.id === currentId"
                    class="current-badge"
                  >当前</span>
                  <div class="model-actions">
                    <Button
                      v-if="item.id !== currentId"
                      size="small"
                      @click="handleSelect(item)"
                    >选择</Button>
                    <Button
                      size="small"
                      @click="handleAddMotion(item)"
                    >
                      <PlayCircleOutlined /> VMD
                    </Button>
                    <Button
                      danger
                      size="small"
                      @click="handleDelete(item)"
                    >
                      <DeleteOutlined />
                    </Button>
                  </div>
                </div>
              </div>
            </List.Item>
          </template>
        </List>

        <div v-else class="empty-hint">
          暂无模型，点击"导入 PMX 模型"开始
        </div>
      </Card>
    </Spin>
  </div>
</template>

<style scoped>
.model3d-config {
  max-width: 600px;
}

.model-row {
  width: 100%;
  display: flex;
  align-items: center;
}

.model-info {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.model-name {
  font-weight: 500;
  min-width: 120px;
}

.current-badge {
  font-size: 11px;
  color: #52c41a;
  border: 1px solid #52c41a;
  border-radius: 4px;
  padding: 0 6px;
}

.model-actions {
  display: flex;
  gap: 6px;
  margin-left: auto;
}

.empty-hint {
  text-align: center;
  color: #aaa;
  padding: 24px;
  font-size: 14px;
}
</style>
