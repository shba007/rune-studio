import { invoke } from '@tauri-apps/api/core'

export const CURSOR_SIZE = 32
export const CURSORS = [
  { kind: 'default', hotspot: [1, 1] },
  { kind: 'link', hotspot: [16, 1] },
  { kind: 'text', hotspot: [16, 16] },
  { kind: 'wait', hotspot: [16, 16] },
  { kind: 'not-allowed', hotspot: [16, 16] },
] as const

export async function installCursorAssets() {
  const images = await Promise.all(
    CURSORS.map(async ({ kind, hotspot }) => {
      const image = new Image()
      image.src = `/cursor/${kind}.svg`
      await image.decode()
      const canvas = document.createElement('canvas')
      canvas.width = canvas.height = CURSOR_SIZE
      const ctx = canvas.getContext('2d')
      if (!ctx) throw new Error('Cursor image renderer unavailable')
      ctx.drawImage(image, 0, 0, CURSOR_SIZE, CURSOR_SIZE)
      return {
        kind,
        hotspot,
        rgba: Array.from(ctx.getImageData(0, 0, CURSOR_SIZE, CURSOR_SIZE).data),
      }
    }),
  )
  await invoke('set_cursor_images', { images })
}