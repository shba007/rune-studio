import { flushPromises, mount } from '@vue/test-utils'
import { reactive, ref } from 'vue'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import type { BackendStatus, ModelCapabilities, Settings } from '@/types/protocol'
import ModelRoleRow from './ModelRoleRow.vue'
import SettingsConnectionModels from './SettingsConnectionModels.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(name: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))
const initial = () => ({
  revision: 0,
  activeId: 'legacy',
  connections: [
    {
      id: 'legacy',
      name: 'Model connection',
      baseUrl: 'http://127.0.0.1:8080',
      active: true,
      hasApiKey: true,
    },
  ],
})
beforeEach(() => {
  harness.invoke
    .mockReset()
    .mockImplementation(async (command) =>
      command === 'connections_state' || command === 'connections_save' ? initial() : null,
    )
})
async function setup() {
  const draft = reactive({
    baseUrl: 'http://127.0.0.1:8080',
    activeConnectionId: 'legacy',
    connectionsRevision: 0,
    connections: [{ id: 'legacy', name: 'Model connection', baseUrl: 'http://127.0.0.1:8080' }],
    model: 'qwen',
    decisionModel: 'fast',
    fastChecks: true,
    decisionUrl: 'http://127.0.0.1:8081',
    models: {
      agent: 'qwen',
      analysis: 'qwen',
      verifier: '',
      voice: '',
      decision: 'fast',
      advisor: '',
    },
  }) as Settings
  const settings = ref({ ...draft, models: { ...draft.models } } as Settings)
  const backend = ref<BackendStatus | null>({
    reachable: true,
    authOk: true,
    missingModels: [],
    modelCapabilities: [],
    latencyMs: 5,
    error: null,
    models: [
      { id: 'qwen', locality: 'local' },
      { id: 'gemma', locality: 'unknown' },
    ] as ModelCapabilities[],
  })
  const refreshBackend = vi.fn<() => Promise<void>>().mockResolvedValue()
  const agent = {
    settings,
    backend,
    refreshBackend,
    hasApiKey: ref(true),
    isRunning: ref(false),
  } as unknown as Agent
  const wrapper = mount(SettingsConnectionModels, {
    props: { agent, draft },
    slots: { default: '<div>Model input resolution</div>' },
  })
  await flushPromises()
  return { wrapper, draft, settings, backend, refreshBackend }
}
it('shows the migrated connection and its actual discovered models', async () => {
  const { wrapper, draft, refreshBackend } = await setup()
  expect(wrapper.text()).toContain('http://127.0.0.1:8080')
  expect(wrapper.text()).toContain('2 models')
  expect(wrapper.text()).toContain('qwen · gemma')
  expect(refreshBackend).not.toHaveBeenCalled()
  await wrapper.get('[aria-label="Open Model connection"]').trigger('click')
  expect(wrapper.text()).toContain('Model roles save automatically')
  expect(wrapper.text()).toContain('Model input resolution')
  expect(draft.decisionUrl).toBe('http://127.0.0.1:8081')
  expect(wrapper.findAllComponents(ModelRoleRow)).toHaveLength(6)
  await wrapper.get('input[aria-label="Search discovered models"]').setValue('gem')
  expect(wrapper.get('[aria-label="Discovered models"] ul').text()).toContain('gemma')
  expect(wrapper.get('[aria-label="Discovered models"] ul').text()).not.toContain('qwen')
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Reload models')!
    .trigger('click')
  expect(refreshBackend).toHaveBeenCalledOnce()
  wrapper.unmount()
})
it('stages edits until Save and preserves shared model roles and Decision URL', async () => {
  const { wrapper, draft } = await setup()
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Edit')!
    .trigger('click')
  await wrapper.get('input[type="url"]').setValue('http://127.0.0.1:9090')
  await wrapper.get('input[type="password"]').setValue('fake-test-key')
  expect(draft.baseUrl).toBe('http://127.0.0.1:8080')
  expect(harness.invoke.mock.calls.some(([name]) => name === 'connections_save')).toBe(false)
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('connections_save', {
    revision: 0,
    id: 'legacy',
    name: 'Model connection',
    baseUrl: 'http://127.0.0.1:9090',
    key: 'fake-test-key',
    clearKey: false,
  })
  expect(draft.models.agent).toBe('qwen')
  expect(draft.decisionUrl).toBe('http://127.0.0.1:8081')
  wrapper.unmount()
})
it('distinguishes unavailable discovery from an empty model list', async () => {
  const { wrapper, backend } = await setup()
  backend.value = { ...backend.value!, reachable: false, error: 'offline', models: [] }
  await flushPromises()
  expect(wrapper.text()).toContain('active model list unavailable')
  expect(wrapper.text()).not.toContain('0 models')
  backend.value = { ...backend.value!, reachable: true, error: null, models: [] }
  await flushPromises()
  expect(wrapper.text()).toContain('0 models')
  await wrapper.get('[aria-label="Open Model connection"]').trigger('click')
  expect(wrapper.text()).toContain('The server reported no models')
  wrapper.unmount()
})
it('saves selected models, additional IDs and the connection output cap', async () => {
  harness.invoke.mockImplementation(async (command) =>
    command === 'connections_state' || command === 'connections_models_save' ? initial() : null,
  )
  const { wrapper } = await setup()
  await wrapper.get('[aria-label="Open Model connection"]').trigger('click')
  const section = wrapper.get('[aria-label="Model selection and limits"]')
  await section
    .findAll('button')
    .find((button) => button.text() === 'Clear')!
    .trigger('click')
  await section.get('textarea').setValue('custom-model')
  const custom = section.findAll('label').find((label) => label.text() === 'custom-model')!
  await custom.get('button[role=switch]').trigger('click')
  await section.get('input[type="number"]').setValue('1000')
  await section
    .findAll('button')
    .find((button) => button.text() === 'Save model settings')!
    .trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('connections_models_save', {
    revision: 0,
    id: 'legacy',
    selectedModels: ['custom-model'],
    additionalModels: ['custom-model'],
    outputLimit: 1000,
  })
  wrapper.unmount()
})
it('requires explicit removal confirmation and requests activation for another saved connection', async () => {
  const two = {
    revision: 3,
    activeId: 'legacy',
    connections: [
      ...initial().connections,
      {
        id: 'conn-two',
        name: 'Second',
        baseUrl: 'http://127.0.0.1:9090',
        active: false,
        hasApiKey: false,
      },
    ],
  }
  harness.invoke.mockImplementation(async (command) =>
    command === 'connections_state' ? two : two,
  )
  const { wrapper } = await setup()
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Make active')!
    .trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('connections_activate', {
    revision: 3,
    id: 'conn-two',
  })
  await wrapper.get('[aria-label="Remove Model connection"]').trigger('click')
  expect(harness.invoke.mock.calls.some(([name]) => name === 'connections_delete')).toBe(false)
  expect(wrapper.text()).toContain('Another saved connection will become active')
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Confirm remove')!
    .trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('connections_delete', { revision: 3, id: 'legacy' })
  wrapper.unmount()
})