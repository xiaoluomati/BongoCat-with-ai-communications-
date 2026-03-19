import type { TrayIconOptions } from '@tauri-apps/api/tray'

import { getName, getVersion } from '@tauri-apps/api/app'
import { Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem } from '@tauri-apps/api/menu'
import { resolveResource } from '@tauri-apps/api/path'
import { TrayIcon } from '@tauri-apps/api/tray'
import { exit, relaunch } from '@tauri-apps/plugin-process'
import { watchDebounced } from '@vueuse/core'
import { watch } from 'vue'
import { range } from 'es-toolkit'

import { showWindow } from '../plugins/window'

import { useSharedMenu } from './useSharedMenu'

import { useCatStore } from '@/stores/cat'
import { useGeneralStore } from '@/stores/general'
import { isMac } from '@/utils/platform'

const TRAY_ID = 'BONGO_CAT_TRAY'

export function useTray() {
  const catStore = useCatStore()
  const generalStore = useGeneralStore()
  const { getSharedMenuItems } = useSharedMenu()

  watch([() => catStore.window.visible, () => catStore.window.passThrough, () => generalStore.appearance.language], () => {
    updateTrayMenu()
  })

  watchDebounced([() => catStore.window.scale, () => catStore.window.opacity], () => {
    updateTrayMenu()
  }, { debounce: 200 })

  const createTray = async () => {
    try {
      const tray = await getTrayById()
      if (tray) return

      const appName = await getName()
      const appVersion = await getVersion()
      const menu = await getTrayMenu()

      // Get icon from resources
      let icon = null
      try {
        const iconPath = isMac ? 'assets/tray-mac.png' : 'assets/tray.png'
        icon = await resolveResource(iconPath)
      } catch (err) {
        console.warn('Could not resolve tray icon:', err)
      }

      const options: TrayIconOptions = {
        menu,
        id: TRAY_ID,
        tooltip: `${appName} v${appVersion}`,
        menuOnLeftClick: true,
      }

      if (icon) {
        options.icon = icon
      }

      await TrayIcon.new(options)
    } catch (e) {
      console.error('Failed to create tray:', e)
    }
  }

  const getTrayById = () => TrayIcon.getById(TRAY_ID)

  const getTrayMenu = async () => {
    const appVersion = await getVersion()
    const sharedItems = await getSharedMenuItems()
    
    const items = [
      ...sharedItems,
      await PredefinedMenuItem.new({ item: 'Separator' }),
      await MenuItem.new({
        id: 'tray-version',
        text: `📋 v${appVersion}`,
        enabled: false,
      }),
      await MenuItem.new({
        id: 'tray-restart',
        text: '🔄 重启应用',
        action: relaunch,
      }),
      await MenuItem.new({
        id: 'tray-quit',
        text: '❌ 退出应用',
        action: () => exit(0),
      }),
    ]

    return Menu.new({ items })
  }

  const updateTrayMenu = async () => {
    try {
      const tray = await getTrayById()
      if (!tray) return
      const menu = await getTrayMenu()
      await tray.setMenu(menu)
    } catch (e) {
      console.error('Failed to update tray menu:', e)
    }
  }

  return { createTray }
}
