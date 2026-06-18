<script setup lang="ts">
import Character from '../character/index.vue'
import Model3D from '../model3d/index.vue'
</script>

<template>
  <div class="models-page">
    <div class="section">
      <div class="section-title">Live2D 角色</div>
      <Character />
    </div>
    <div class="section">
      <div class="section-title">3D 模型</div>
      <Model3D />
    </div>
  </div>
</template>

<style scoped>
.models-page {
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #666;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(0,0,0,0.06);
}
</style>
