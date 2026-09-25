import { ref } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { MediaItem } from '@/types/media'

export function useMediaScanner() {
  const isScanning = ref(false)
  const scanError = ref<string | null>(null)

  async function pickDirectory(): Promise<string | null> {
    try {
      const selected = await invoke<string | null>('pick_directory')
      return selected
    } catch (err: unknown) {
      console.error('[useMediaScanner] pickDirectory error:', err)
      return null
    }
  }

  async function scanDirectory(path: string, recursive: boolean = true): Promise<MediaItem[]> {
    if (!path.trim()) return []

    isScanning.value = true
    scanError.value = null

    try {
      const items = await invoke<MediaItem[]>('scan_directory', {
        path,
        recursive,
      })

      return items.map((item) => ({
        ...item,
        thumbnailUrl: convertFileSrc(item.path),
      }))
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      console.error('[useMediaScanner] scanDirectory error:', msg)
      scanError.value = msg
      return []
    } finally {
      isScanning.value = false
    }
  }

  async function getMediaBase64(path: string): Promise<string> {
    return await invoke<string>('get_media_base64', { path })
  }

  async function executeFileActions(
    actions: {
      actionType: 'MOVE' | 'COPY'
      sourcePath: string
      targetDirectory: string
      targetFileName?: string
    }[],
  ): Promise<{ successCount: number; errors: string[] }> {
    return await invoke<{ successCount: number; errors: string[] }>('execute_file_actions', {
      actions,
    })
  }

  return {
    isScanning,
    scanError,
    pickDirectory,
    scanDirectory,
    getMediaBase64,
    executeFileActions,
  }
}