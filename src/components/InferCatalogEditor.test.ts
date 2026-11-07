import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import InferCatalogEditor from './InferCatalogEditor.vue'

const api = vi.hoisted(() =>
  vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
)
vi.mock('@tauri-apps/api/core', () => ({ invoke: api }))

it('preserves unrelated catalog fields and stages role changes before saving', async () => {
  const initial = {
    schema_version: 3,
    models: [
      {
        id: 'one',
        name: 'One',
        modality: 'Text',
        local: { path: '/one' },
        engine: { supported: ['llama-server'] },
      },
    ],
    aliases: { alias: 'one' },
  }
  api
    .mockReset()
    .mockImplementation(async (command, args) =>
      command === 'infer_catalog_read'
        ? { revision: 'old', catalog: JSON.stringify(initial) }
        : (args?.document as object),
    )
  const wrapper = mount(InferCatalogEditor)
  const button = (text: string) =>
    wrapper.findAll('button').find((item) => item.text().includes(text))!
  await button('Load catalog').trigger('click')
  await flushPromises()
  await wrapper.find('select').setValue('one')
  const visual = wrapper.findAll('label').find((label) => label.text().includes('visual depth'))!
  await visual.find('select').setValue('one')
  expect(api).toHaveBeenCalledTimes(1)
  await button('Save catalog').trigger('click')
  await flushPromises()
  const doc = api.mock.calls[1]![1]!.document as { revision: string; catalog: string }
  expect(doc.revision).toBe('old')
  expect(JSON.parse(doc.catalog)).toMatchObject({
    aliases: { alias: 'one' },
    roles: { agent: 'one', visual_depth: 'one' },
    models: initial.models,
  })
})

it('adds a remote model using form fields and saves only when staged', async () => {
  api
    .mockReset()
    .mockImplementation(async (command, args) =>
      command === 'infer_catalog_read'
        ? { revision: 'v1', catalog: JSON.stringify({ schema_version: 3, models: [] }) }
        : args?.document,
    )
  const wrapper = mount(InferCatalogEditor)
  const button = (text: string) =>
    wrapper.findAll('button').find((item) => item.text().includes(text))!
  await button('Load catalog').trigger('click')
  await flushPromises()
  await button('Add model').trigger('click')
  await wrapper.get('input[aria-label="Model ID"]').setValue('cloud')
  await wrapper.get('input[aria-label="Model name"]').setValue('Cloud')
  await wrapper.get('input[aria-label="Provider"]').setValue('openai')
  await wrapper.get('input[aria-label="Provider model ID"]').setValue('upstream')
  expect(button('Save catalog').attributes('disabled')).toBeDefined()
  await button('Stage model').trigger('click')
  await button('Save catalog').trigger('click')
  await flushPromises()
  const document = api.mock.calls[1]![1]!.document as { catalog: string }
  expect(JSON.parse(document.catalog).models[0]).toMatchObject({
    id: 'cloud',
    name: 'Cloud',
    remote: { provider: 'openai', upstream_model: 'upstream', api_key: '{{PROVIDER_API_KEY}}' },
  })
})

it('preserves a model referenced by an alias when removal is requested', async () => {
  api.mockReset().mockResolvedValue({
    revision: 'v1',
    catalog: JSON.stringify({
      schema_version: 3,
      models: [{ id: 'one', name: 'One' }],
      aliases: { alias: 'one' },
    }),
  })
  const wrapper = mount(InferCatalogEditor)
  const button = (text: string) =>
    wrapper.findAll('button').find((item) => item.text().includes(text))!
  await button('Load catalog').trigger('click')
  await flushPromises()
  await button('Remove').trigger('click')
  expect(wrapper.text()).toContain('Clear this model from its roles and aliases')
  expect(wrapper.text()).toContain('One (one)')
})

it('displays and clears model-level role assignments without changing other roles', async () => {
  api.mockReset().mockImplementation(async (command, args) =>
    command === 'infer_catalog_read'
      ? {
          revision: 'r1',
          connectionId: 'infer',
          connectionRevision: 1,
          catalog: JSON.stringify({
            schema_version: 3,
            models: [{ id: 'one', name: 'One', roles: ['voice', 'visual_depth'] }],
          }),
        }
      : args?.document,
  )
  const wrapper = mount(InferCatalogEditor)
  const button = (text: string) =>
    wrapper.findAll('button').find((item) => item.text().includes(text))!
  await button('Load catalog').trigger('click')
  await flushPromises()
  const depth = wrapper
    .findAll('label')
    .find((label) => label.text().includes('visual depth'))!
    .find('select')
  expect((depth.element as HTMLSelectElement).value).toBe('one')
  await depth.setValue('')
  await button('Save catalog').trigger('click')
  await flushPromises()
  const document = api.mock.calls[1]![1]!.document as { catalog: string }
  expect(JSON.parse(document.catalog).models[0].roles).toEqual(['voice'])
  wrapper.unmount()
})