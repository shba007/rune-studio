import { ref } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import type { MediaItem } from '@/types/media'

export interface ThumbnailResult {
  id: string
  thumbnailPath: string
}

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
      const items = await invoke<Array<MediaItem & { thumbnailPath?: string }>>('scan_directory', {
        path,
        recursive,
      })

      return items.map((item) => ({
        ...item,
        thumbnailPath: item.thumbnailPath,
        thumbnailUrl: item.thumbnailPath
          ? convertFileSrc(item.thumbnailPath)
          : convertFileSrc(item.path),
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

  async function generateThumbnails(
    items: MediaItem[],
    folderPath: string,
  ): Promise<ThumbnailResult[]> {
    if (items.length === 0 || !folderPath.trim()) return []
    try {
      return await invoke<ThumbnailResult[]>('generate_thumbnails', {
        items,
        folderPath,
      })
    } catch (err) {
      console.warn('[useMediaScanner] generateThumbnails error:', err)
      return []
    }
  }

  async function uploadMediaToS3(filePath: string, bucketUrl: string): Promise<string> {
    return await invoke<string>('upload_media_to_s3', { filePath, bucketUrl })
  }

  async function getMediaBase64(path: string): Promise<string> {
    return await invoke<string>('get_media_base64', { path })
  }

  async function saveArtifactToDisk(
    folderPath: string,
    fileHash: string,
    intent: string,
    data: string,
  ): Promise<void> {
    await invoke('save_artifact_to_disk', { folderPath, fileHash, intent, data })
  }

  async function loadArtifactFromDisk(
    folderPath: string,
    fileHash: string,
    intent: string,
  ): Promise<string | null> {
    return await invoke<string | null>('load_artifact_from_disk', { folderPath, fileHash, intent })
  }

  async function clearDirectoryCache(folderPath: string): Promise<void> {
    await invoke('clear_directory_cache', { folderPath })
  }

  async function deleteArtifactFromDisk(
    folderPath: string,
    fileHash: string,
    intent: string,
  ): Promise<void> {
    await invoke('delete_artifact_from_disk', { folderPath, fileHash, intent })
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
    generateThumbnails,
    uploadMediaToS3,
    getMediaBase64,
    saveArtifactToDisk,
    loadArtifactFromDisk,
    clearDirectoryCache,
    deleteArtifactFromDisk,
    executeFileActions,
  }
}