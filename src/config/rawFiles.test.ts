import { describe, expect, it } from 'vite-plus/test'
import { inspectRawFolder } from './rawFiles'

function file(path: string) {
  const result = new File(['x'], path.split('/').at(-1)!)
  Object.defineProperty(result, 'webkitRelativePath', { value: path })
  return result
}

describe('RAW folder inspection', () => {
  it('blocks both sidecar naming conventions in the same directory only', () => {
    const files = [
      'shoot/a.ARW',
      'shoot/A.XMP',
      'shoot/b.arw',
      'shoot/b.ARW.xmp',
      'shoot/c.ARW',
      'other/c.xmp',
    ].map(file)
    expect(inspectRawFolder(files).map((item) => [item.path, item.blocked])).toEqual([
      ['shoot/a.ARW', true],
      ['shoot/b.arw', true],
      ['shoot/c.ARW', false],
    ])
  })
  it('refuses files without folder paths', () => {
    expect(() => inspectRawFolder([file('')])).toThrow('Choose a folder')
  })
})