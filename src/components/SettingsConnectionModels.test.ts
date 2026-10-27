import { flushPromises, mount } from '@vue/test-utils'
import { reactive, ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import type { BackendStatus, ModelCapabilities, Settings } from '@/types/protocol'
import ModelRoleRow from './ModelRoleRow.vue'
import SettingsConnectionModels from './SettingsConnectionModels.vue'

function setup() {
  const draft = reactive({
    baseUrl: 'http://127.0.0.1:8080',
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
  const saveApiKey = vi.fn<(key: string) => Promise<void>>().mockResolvedValue()
  const agent = {
    settings,
    backend,
    refreshBackend,
    saveApiKey,
    hasApiKey: ref(true),
  } as unknown as Agent
  const wrapper = mount(SettingsConnectionModels, {
    props: { agent, draft, apiKey: '' },
    slots: { default: '<div>Model input resolution</div>' },
  })
  return { wrapper, draft, settings, backend, refreshBackend, saveApiKey }
}
it('shows the existing connection and opens its details without changing settings', async () => {
  const { wrapper, draft, refreshBackend } = setup()
  expect(wrapper.text()).toContain('http://127.0.0.1:8080')
  expect(wrapper.text()).toContain('2 models')
  expect(wrapper.text()).toContain('qwen · gemma')
  expect(wrapper.find('input[aria-label="Base URL"]').exists()).toBe(false)
  expect(refreshBackend).not.toHaveBeenCalled()
  await wrapper.get('[aria-label="Edit model connection"]').trigger('click')
  expect(wrapper.text()).toContain('Changes save automatically')
  expect(wrapper.text()).toContain('Model input resolution')
  expect((wrapper.get('input[aria-label="Base URL"]').element as HTMLInputElement).value).toBe(
    draft.baseUrl,
  )
  expect(draft.decisionUrl).toBe('http://127.0.0.1:8081')
  expect(wrapper.findAllComponents(ModelRoleRow)).toHaveLength(6)
  await wrapper.get('input[aria-label="Search discovered models"]').setValue('gem')
  const list = wrapper.get('[aria-label="Discovered models"] ul')
  expect(list.text()).toContain('gemma')
  expect(list.text()).not.toContain('qwen')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Reload models')!
    .trigger('click')
  expect(refreshBackend).toHaveBeenCalledOnce()
  wrapper.unmount()
})
it('preserves endpoint, roles, decision URL and API key behavior when edited', async () => {
  const { wrapper, draft, saveApiKey } = setup()
  await wrapper.get('[aria-label="Edit model connection"]').trigger('click')
  await wrapper.get('input[aria-label="Base URL"]').setValue('http://127.0.0.1:9090')
  expect(draft.baseUrl).toBe('http://127.0.0.1:9090')
  expect(wrapper.text()).toContain('Model list unavailable')
  const row = wrapper
    .findAllComponents(ModelRoleRow)
    .find((item) => item.props('roleKey') === 'agent')!
  row.vm.$emit('update:modelValue', 'gemma')
  await flushPromises()
  expect(draft.models.agent).toBe('gemma')
  expect(draft.model).toBe('gemma')
  expect(draft.decisionUrl).toBe('http://127.0.0.1:8081')
  await wrapper.get('input[aria-label="API key"]').setValue('replacement')
  expect(wrapper.emitted('update:apiKey')?.at(-1)).toEqual(['replacement'])
  expect(wrapper.emitted('commit-key')).toBeTruthy()
  expect(saveApiKey).not.toHaveBeenCalled()
  wrapper.unmount()
})
it('distinguishes unavailable discovery from an empty model list', async () => {
  const { wrapper, backend } = setup()
  backend.value = { ...backend.value!, reachable: false, error: 'offline', models: [] }
  await flushPromises()
  expect(wrapper.text()).toContain('Model list unavailable')
  expect(wrapper.text()).not.toContain('0 models')
  backend.value = { ...backend.value!, reachable: true, error: null, models: [] }
  await flushPromises()
  expect(wrapper.text()).toContain('0 models')
  await wrapper.get('[aria-label="Edit model connection"]').trigger('click')
  expect(wrapper.text()).toContain('The server reported no models')
  wrapper.unmount()
})