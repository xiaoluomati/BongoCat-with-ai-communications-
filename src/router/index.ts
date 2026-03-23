import type { RouteRecordRaw } from 'vue-router'

import { createRouter, createWebHashHistory } from 'vue-router'

import Chat from '../pages/chat/index.vue'
import Comprehensive_Function from '../pages/comprehensive_function/index.vue'
import Main from '../pages/main/index.vue'
import Preference from '../pages/preference/index.vue'

const routes: Readonly<RouteRecordRaw[]> = [
  {
    path: '/',
    component: Main,
  },
  {
    path: '/preference',
    component: Preference,
  },
  {
    path: '/comprehensive_function',
    component: Comprehensive_Function,
  },
  {
    path: '/chat',
    component: Chat,
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
