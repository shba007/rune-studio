export interface RawFile {
  file: File
  path: string
  blocked: boolean
}

export function inspectRawFolder(files: File[]): RawFile[] {
  if (files.some((file) => !file.webkitRelativePath))
    throw new Error('Choose a folder so Studio can check every RAW file for XMP sidecars.')
  const paths = new Set(files.map((file) => file.webkitRelativePath.toLowerCase()))
  const raws = files.filter((file) => file.name.toLowerCase().endsWith('.arw'))
  if (raws.length > 100) throw new Error('Choose a folder containing no more than 100 ARW files.')
  if (!raws.length) throw new Error('No ARW files found in this folder.')
  return raws.map((file) => {
    const path = file.webkitRelativePath
    const lower = path.toLowerCase()
    return {
      file,
      path,
      blocked: paths.has(`${lower}.xmp`) || paths.has(`${lower.slice(0, -4)}.xmp`),
    }
  })
}

export function readRawFile(file: File): Promise<string> {
  if (file.size > 150 * 1024 * 1024)
    return Promise.reject(new Error('RAW file exceeds the 150 MB limit'))
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () =>
      typeof reader.result === 'string'
        ? resolve(reader.result.split(',', 2)[1] ?? '')
        : reject(new Error('Could not read RAW file'))
    reader.onerror = () => reject(new Error('Could not read RAW file'))
    reader.readAsDataURL(file)
  })
}