export const MAX_FILE_BYTES = 25 * 1024 * 1024
export const MAX_FILE_EDGE = 4096

export function checkImageFile(file: { type: string; size: number }): string | null {
  if (!file.type.startsWith('image/')) return 'Choose an image file.'
  if (file.size > MAX_FILE_BYTES) return 'Choose an image under 25 MB.'
  return null
}

export function fitEdge(width: number, height: number, limit = MAX_FILE_EDGE) {
  const longest = Math.max(width, height)
  if (longest <= limit) return { width, height }
  const scale = limit / longest
  return { width: Math.round(width * scale), height: Math.round(height * scale) }
}

export async function readImageAsPng(file: File): Promise<string> {
  const problem = checkImageFile(file)
  if (problem) throw new Error(problem)
  const bitmap = await createImageBitmap(file).catch(() => {
    throw new Error('That image could not be read.')
  })
  const { width, height } = fitEdge(bitmap.width, bitmap.height)
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d')
  if (!context) throw new Error('That image could not be read.')
  context.drawImage(bitmap, 0, 0, width, height)
  bitmap.close()
  return canvas.toDataURL('image/png')
}