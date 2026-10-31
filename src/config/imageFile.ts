export const MAX_FILE_BYTES = 25 * 1024 * 1024
export const MAX_FILE_EDGE = 2560

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

const UNREADABLE = 'That image could not be read.'

function readDataUri(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () =>
      typeof reader.result === 'string' ? resolve(reader.result) : reject(new Error(UNREADABLE))
    reader.onerror = () => reject(new Error(UNREADABLE))
    reader.readAsDataURL(blob)
  })
}

async function encodePng(bitmap: ImageBitmap, width: number, height: number): Promise<Blob> {
  if (typeof OffscreenCanvas === 'undefined') {
    const canvas = document.createElement('canvas')
    canvas.width = width
    canvas.height = height
    canvas.getContext('2d')?.drawImage(bitmap, 0, 0, width, height)
    return new Promise((resolve, reject) =>
      canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error(UNREADABLE))), 'image/png'),
    )
  }
  const canvas = new OffscreenCanvas(width, height)
  canvas.getContext('2d')?.drawImage(bitmap, 0, 0, width, height)
  return canvas.convertToBlob({ type: 'image/png' })
}

export async function readImageAsPng(file: File): Promise<string> {
  const problem = checkImageFile(file)
  if (problem) throw new Error(problem)
  const probe = await createImageBitmap(file).catch(() => {
    throw new Error(UNREADABLE)
  })
  const { width, height } = fitEdge(probe.width, probe.height)
  const resized = width !== probe.width || height !== probe.height
  probe.close()
  if (file.type === 'image/png' && !resized) return readDataUri(file)
  const bitmap = await createImageBitmap(file, {
    resizeWidth: width,
    resizeHeight: height,
    resizeQuality: 'high',
  }).catch(() => {
    throw new Error(UNREADABLE)
  })
  try {
    return await readDataUri(await encodePng(bitmap, width, height))
  } finally {
    bitmap.close()
  }
}