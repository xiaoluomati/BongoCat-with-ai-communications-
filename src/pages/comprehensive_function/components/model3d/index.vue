<script setup lang="ts">
import { DeleteOutlined, PlusOutlined, PlayCircleOutlined } from '@ant-design/icons-vue'
import { Button, Card, Modal, message, Spin, Popconfirm } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { ref, onMounted, computed } from 'vue'
import { useModel3DStore, type Model3DInfo } from '@/stores/model3d'

const store = useModel3DStore()
const loading = ref(false)

function notify3DWindow() {
  emit('model3d-updated', { currentId: store.currentModelId }).catch(() => {})
}

const models = computed(() => store.models)
const currentId = computed(() => store.currentModelId)

onMounted(async () => { await loadModels() })

async function loadModels() {
  loading.value = true
  try {
    const list = await invoke<Model3DInfo[]>('list_3d_models')
    store.setModels(list)
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
    notify3DWindow()
  } catch (e) {
    message.error('导入失败: ' + String(e))
  } finally {
    loading.value = false
  }
}

function handleSelect(model: Model3DInfo) {
  store.selectModel(model.id)
  message.success(`已选择: ${model.name}`)
  notify3DWindow()
}

async function handleDelete(model: Model3DInfo) {
  try {
    await invoke('remove_3d_model', { id: model.id })
    if (currentId.value === model.id) store.selectModel('')
    message.success('已删除')
    await loadModels()
    notify3DWindow()
  } catch (e) {
    message.error('删除失败: ' + String(e))
  }
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
  <div class="model3d-section">
    <div class="section-header">
      <span class="section-title">3D 模型</span>
      <Button type="primary" size="small" @click="handleImport">
        <PlusOutlined /> 导入
      </Button>
    </div>

    <Spin :spinning="loading">
      <div v-if="models.length > 0" class="model-grid">
        <Card
          v-for="item in models"
          :key="item.id"
          hoverable
          size="small"
          :class="{ 'selected': item.id === currentId }"
          @click="handleSelect(item)"
        >
          <template #cover>
            <div class="model-thumb">
              <div class="thumb-placeholder">PMX</div>
              <div v-if="item.id === currentId" class="current-tag">当前</div>
            </div>
          </template>

          <Card.Meta :title="item.name" />

          <template #actions>
            <PlayCircleOutlined
              key="motion"
              title="添加动作"
              @click.stop="handleAddMotion(item)"
            />
            <Popconfirm
              title="确定删除此模型？"
              @confirm="handleDelete(item)"
            >
              <DeleteOutlined key="delete" @click.stop />
            </Popconfirm>
          </template>
        </Card>
      </div>

      <div v-else class="empty-hint">
        暂无 3D 模型，点击"导入"添加
      </div>
    </Spin>
  </div>
</template>

<style scoped>
.model3d-section {
  margin-top: 8px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
}

.model-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 16px;
}

.model-thumb {
  height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f5f5f5;
  position: relative;
  overflow: hidden;
}

.thumb-placeholder {
  font-size: 24px;
  font-weight: 700;
  color: #ccc;
}

.current-tag {
  position: absolute;
  top: 6px;
  right: 6px;
  font-size: 11px;
  color: #fff;
  background: #52c41a;
  border-radius: 4px;
  padding: 2px 8px;
}

.selected {
  border-color: #1677ff;
}

.empty-hint {
  text-align: center;
  color: #aaa;
  padding: 32px;
  font-size: 14px;
}
</style>
