import { Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem } from '@tauri-apps/api/menu'
import { invoke } from '@tauri-apps/api/core'
import { range } from 'es-toolkit'

import { showWindow } from '@/plugins/window'
import { useCatStore } from '@/stores/cat'

export function useSharedMenu() {
  const catStore = useCatStore()

  // Get scale menu items
  const getScaleItems = async () => {
    const options = range(50, 151, 25)
    const items = []
    
    for (const item of options) {
      items.push(await MenuItem.new({
        id: `scale-${item}`,
        text: `${item}%`,
        action: () => { catStore.window.scale = item }
      }))
    }
    
    if (!options.includes(catStore.window.scale)) {
      items.unshift(await MenuItem.new({
        id: 'scale-current',
        text: `${catStore.window.scale}%`,
        enabled: false,
      }))
    }
    
    return items
  }

  // Get opacity menu items
  const getOpacityItems = async () => {
    const options = range(25, 101, 25)
    const items = []
    
    for (const item of options) {
      items.push(await MenuItem.new({
        id: `opacity-${item}`,
        text: `${item}%`,
        action: () => { catStore.window.opacity = item }
      }))
    }
    
    if (!options.includes(catStore.window.opacity)) {
      items.unshift(await MenuItem.new({
        id: 'opacity-current',
        text: `${catStore.window.opacity}%`,
        enabled: false,
      }))
    }
    
    return items
  }

  const getSharedMenuItems = async (): Promise<any[]> => {
    const chatItem = await MenuItem.new({
      id: 'chat',
      text: '💬 聊天',
      action: async () => { await invoke('show_chat_window') },
    })

    const compItem = await MenuItem.new({
      id: 'comprehensive',
      text: '📁 综合功能',
      action: async () => { 
        await invoke('activate_window', { windowLabel: 'comprehensive_function' })
      },
    })

    const prefItem = await MenuItem.new({
      id: 'preference',
      text: '⚙️ 偏好设置',
      action: async () => { 
        await invoke('activate_window', { windowLabel: 'preference' })
      },
    })

    const toggleItem = await MenuItem.new({
      id: 'toggle',
      text: catStore.window.visible ? '👀 隐藏宠物' : '👀 显示宠物',
      action: () => { catStore.window.visible = !catStore.window.visible },
    })

    const passItem = await CheckMenuItem.new({
      id: 'passthrough',
      text: '🔲 鼠标穿透',
      checked: catStore.window.passThrough,
      action: () => { catStore.window.passThrough = !catStore.window.passThrough },
    })

    const scaleItems = await getScaleItems()
    const opacityItems = await getOpacityItems()

    const scaleSubmenu = await Submenu.new({
      id: 'scale-menu',
      text: '📐 窗口大小',
      items: scaleItems,
    })

    const opacitySubmenu = await Submenu.new({
      id: 'opacity-menu',
      text: '🌫️ 透明度',
      items: opacityItems,
    })

    const sep = await PredefinedMenuItem.new({ item: 'Separator' })

    return [chatItem, compItem, prefItem, toggleItem, sep, passItem, scaleSubmenu, opacitySubmenu]
  }

  const getSharedMenu = async () => {
    const items = await getSharedMenuItems()
    return Menu.new({ items })
  }

  return { getSharedMenu, getSharedMenuItems }
}
