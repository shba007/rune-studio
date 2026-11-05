import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import PlaygroundRawDevelop from './PlaygroundRawDevelop.vue'

const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

function file(name: string) {
  const result = new File(['raw'], name)
  Object.defineProperty(result, 'webkitRelativePath', { value: `shoot/${name}` })
  return result
}

it('blocks RAW files with sidecars and develops the chosen eligible image', async () => {
  invoke.mockResolvedValue({ path: 'C:\\output\\developed.png', width: 4672, height: 7008 })
  const wrapper = mount(PlaygroundRawDevelop)
  const input = wrapper.get('#raw-develop-folder')
  Object.defineProperty(input.element, 'files', {
    value: [file('a.ARW'), file('b.ARW'), file('b.xmp')],
  })
  await input.trigger('change')
  expect(wrapper.text()).toContain('1 blocked by XMP edits')
  expect(wrapper.get('option[value="shoot/b.ARW"]').attributes('disabled')).toBeDefined()
  await wrapper.get('select').setValue('shoot/a.ARW')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Develop 16-bit PNG')!
    .trigger('click')
  await vi.waitFor(() =>
    expect(invoke).toHaveBeenCalledWith('portfolio_develop_raw', { raw: expect.any(String) }),
  )
  await flushPromises()
  expect(wrapper.text()).toContain('4672 × 7008')
})

it('clears a previous selection when the new folder cannot be inspected', async () => {
  const wrapper = mount(PlaygroundRawDevelop)
  const input = wrapper.get('#raw-develop-folder')
  Object.defineProperty(input.element, 'files', { value: [file('a.ARW')], configurable: true })
  await input.trigger('change')
  await wrapper.get('select').setValue('shoot/a.ARW')
  Object.defineProperty(input.element, 'files', { value: [new File(['raw'], 'b.ARW')] })
  await input.trigger('change')
  expect(wrapper.find('option[value="shoot/a.ARW"]').exists()).toBe(false)
  expect(wrapper.text()).toContain('folder')
})