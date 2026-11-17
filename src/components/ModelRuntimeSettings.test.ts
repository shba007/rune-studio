import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import ModelRuntimeSettings from './ModelRuntimeSettings.vue'

const api = vi.hoisted(() =>
  vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
)
vi.mock('@tauri-apps/api/core', () => ({ invoke: api }))

const catalog = () => ({
  schema_version: 3,
  models: [
    {
      id: 'holo',
      name: 'Holo',
      max_context_length: 32768,
      local: { path: 'C:/m.gguf', context_length: -1, gpu_layers: 99, args: '-fa on' },
      sampling: { temperature: 0, max_tokens: 512 },
      extra: { mmproj_path: 'C:/mm.gguf' },
    },
    { id: 'cloud', name: 'Cloud', remote: { provider: 'x', upstream_model: 'y' } },
  ],
  roles: { agent: 'holo' },
})
const doc = (value: object, revision = 'r1') => ({
  revision,
  catalog: JSON.stringify(value),
  connectionId: 'c',
  connectionRevision: 7,
})

function serve(status = 'loaded') {
  let current = catalog()
  let revision = 1
  api.mockReset().mockImplementation(async (command, args) => {
    if (command === 'infer_catalog_read') return doc(current, `r${revision}`)
    if (command === 'infer_catalog_write') {
      const written = args!.document as { catalog: string; revision: string }
      if (written.revision !== `r${revision}`) throw new Error('stale revision')
      current = JSON.parse(written.catalog)
      revision += 1
      return doc(current, `r${revision}`)
    }
    return undefined
  })
  return mount(ModelRuntimeSettings, { props: { models: [{ id: 'holo', status }] } })
}
const writes = () => api.mock.calls.filter(([command]) => command === 'infer_catalog_write')
const button = (wrapper: ReturnType<typeof serve>, text: string) =>
  wrapper.findAll('button').find((item) => item.text() === text)!
const input = (wrapper: ReturnType<typeof serve>, label: string) =>
  wrapper.get(`input[aria-label="${label}"]`)

it('shows the agent model with Auto context and its sampling, and nothing to save yet', async () => {
  const wrapper = serve()
  await flushPromises()
  expect(wrapper.text()).toContain("Auto uses the model's maximum (32768 tokens)")
  expect((input(wrapper, 'Max output tokens').element as HTMLInputElement).value).toBe('512')
  expect((input(wrapper, 'Top-p').element as HTMLInputElement).value).toBe('')
  expect(wrapper.find('input[aria-label="Context size (tokens)"]').exists()).toBe(false)
  expect(button(wrapper, 'Save').attributes('disabled')).toBeDefined()
  wrapper.unmount()
})

it('saves a manual context size and sampling and keeps the rest of the model entry', async () => {
  const wrapper = serve()
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((item) => item.text() === 'Manual')!
    .trigger('click')
  await input(wrapper, 'Context size (tokens)').setValue('8192')
  await input(wrapper, 'Max output tokens').setValue('2048')
  await button(wrapper, 'Save').trigger('click')
  await flushPromises()
  expect(writes()).toHaveLength(1)
  const saved = JSON.parse((writes()[0]![1]!.document as { catalog: string }).catalog)
  expect(saved.models[0].local).toEqual({
    path: 'C:/m.gguf',
    context_length: 8192,
    gpu_layers: 99,
    args: '-fa on',
  })
  expect(saved.models[0].sampling).toEqual({ temperature: 0, max_tokens: 2048 })
  expect(saved.models[0].extra).toEqual({ mmproj_path: 'C:/mm.gguf' })
  expect(saved.roles).toEqual({ agent: 'holo' })
  expect(wrapper.text()).toContain('Press Apply to reload')
  expect(button(wrapper, 'Save').attributes('disabled')).toBeDefined()
  wrapper.unmount()
})

it('refuses an invalid value, names the problem and writes nothing', async () => {
  const wrapper = serve()
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((item) => item.text() === 'Manual')!
    .trigger('click')
  await input(wrapper, 'Context size (tokens)').setValue('99999')
  await input(wrapper, 'Temperature').setValue('9')
  expect(wrapper.get('[role="alert"]').text()).toContain('at most 32768 tokens')
  expect(wrapper.get('[role="alert"]').text()).toContain('Use 0 to 5')
  expect(button(wrapper, 'Save').attributes('disabled')).toBeDefined()
  expect(button(wrapper, 'Apply').attributes('disabled')).toBeDefined()
  expect(writes()).toHaveLength(0)
  wrapper.unmount()
})

it('Apply saves pending changes, then unloads and reloads the model', async () => {
  const wrapper = serve()
  await flushPromises()
  await input(wrapper, 'Top-k').setValue('40')
  await button(wrapper, 'Apply').trigger('click')
  await flushPromises()
  const commands = api.mock.calls.map(([command, args]) => [command, args?.load])
  expect(commands.slice(1)).toEqual([
    ['infer_catalog_write', undefined],
    ['infer_model_residency', false],
    ['infer_model_residency', true],
  ])
  expect(wrapper.text()).toContain('Reloaded Holo with the saved settings')
  wrapper.unmount()
})

it('cannot Apply a model that is not loaded and says when the settings will apply', async () => {
  const wrapper = serve('idle')
  await flushPromises()
  await input(wrapper, 'Top-k').setValue('40')
  expect(button(wrapper, 'Apply').attributes('disabled')).toBeDefined()
  expect(wrapper.text()).toContain('apply when it loads')
  await button(wrapper, 'Save').trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('apply the next time the model loads')
  wrapper.unmount()
})

it('keeps a failed save visible with the draft intact', async () => {
  const wrapper = serve()
  await flushPromises()
  api.mockImplementation(async (command) => {
    if (command === 'infer_catalog_write')
      throw 'The Infer catalog changed or could not be saved; reload before retrying'
    return undefined
  })
  await input(wrapper, 'Top-k').setValue('40')
  await button(wrapper, 'Save').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('reload before retrying')
  expect((input(wrapper, 'Top-k').element as HTMLInputElement).value).toBe('40')
  wrapper.unmount()
})

it('hides the context window for a remote model but still edits its sampling', async () => {
  const wrapper = serve()
  await flushPromises()
  const choice = wrapper.findAll('button').find((item) => item.text().includes('Holo'))!
  await choice.trigger('click')
  const cloud = wrapper.findAll('button[role="option"]').find((item) => item.text() === 'Cloud')
  await cloud!.trigger('click')
  await flushPromises()
  expect(wrapper.text()).not.toContain('Context window')
  expect(wrapper.find('input[aria-label="Temperature"]').exists()).toBe(true)
  wrapper.unmount()
})