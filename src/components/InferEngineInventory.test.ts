import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import InferEngineInventory from './InferEngineInventory.vue'

const api = vi.hoisted(() => ({ invoke: vi.fn<() => Promise<unknown>>() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))

it('loads and displays the active Infer server inventory', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      { name: 'llama-server', installed: true, managed: true, version: '1.2.3' },
      { name: 'whisper-cpp', installed: false, managed: false, version: 'Not installed' },
    ],
  })
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('infer_engines')
  expect(wrapper.text()).toContain('llama-server')
  expect(wrapper.text()).toContain('Managed by Infer')
  expect(wrapper.text()).toContain('Not installed')
})

it('shows an Infer compatibility error without stale engines', async () => {
  api.invoke.mockRejectedValueOnce('Update Infer and retry.')
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Update Infer and retry.')
})

it('checks available releases through Infer', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      {
        name: 'needle',
        installed: true,
        managed: true,
        version: 'old',
        release_check_available: true,
      },
    ],
  })
  api.invoke.mockResolvedValueOnce({ latest: 'new', detail: 'Reviewed registry build new' })
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  await wrapper.findAll('button')[1]!.trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('infer_engine_release', { name: 'needle' })
  expect(wrapper.text()).toContain('Reviewed registry build new')
})

it('shows release check failures', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      {
        name: 'needle',
        installed: true,
        managed: true,
        version: 'old',
        release_check_available: true,
      },
    ],
  })
  api.invoke.mockRejectedValueOnce('Infer could not check the engine release (502)')
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  await wrapper.findAll('button')[1]!.trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Infer could not check the engine release (502)')
})

it('starts installation through Infer and displays the running job', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      {
        name: 'needle',
        installed: false,
        managed: false,
        version: 'Not installed',
        installation_available: true,
      },
    ],
  })
  api.invoke.mockResolvedValueOnce({
    name: 'needle',
    status: 'running',
    message: 'Installing engine',
  })
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  await wrapper.findAll('button')[1]!.trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('infer_engine_installation', {
    name: 'needle',
    start: true,
  })
  expect(wrapper.text()).toContain('Installing engine')
  expect(wrapper.text()).toContain('Installing…')
  wrapper.unmount()
})

it('starts an update through Infer and displays the running job', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      { name: 'needle', installed: true, managed: true, version: 'old', update_available: true },
    ],
  })
  api.invoke.mockResolvedValueOnce({
    name: 'needle',
    operation: 'update',
    status: 'running',
    message: 'Checking and updating engine',
  })
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  await wrapper.findAll('button')[1]!.trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('infer_engine_update', { name: 'needle', start: true })
  expect(wrapper.text()).toContain('Updating…')
  wrapper.unmount()
})

it('restores an Infer update job when inventory is reloaded', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      { name: 'needle', installed: true, managed: true, version: 'old', update_available: true },
    ],
    installation: {
      name: 'needle',
      operation: 'update',
      status: 'running',
      message: 'Updating engine',
    },
  })
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Updating…')
  wrapper.unmount()
})

it('requests cancellation through the active Infer connection', async () => {
  api.invoke.mockResolvedValueOnce({
    engines: [
      { name: 'needle', installed: true, managed: true, version: 'old', update_available: true },
    ],
    installation: {
      name: 'needle',
      operation: 'update',
      status: 'running',
      message: 'Updating engine',
    },
  })
  api.invoke.mockResolvedValueOnce({
    name: 'needle',
    operation: 'update',
    status: 'running',
    message: 'Cancelling engine operation; waiting for current step',
  })
  const wrapper = mount(InferEngineInventory, {
    props: { connectionId: 'fixture', connectionRevision: 3 },
  })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Cancel')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('infer_engine_cancel', {
    name: 'needle',
    operation: 'update',
    connectionId: 'fixture',
    connectionRevision: 3,
  })
  expect(wrapper.text()).toContain('Cancelling engine operation')
  expect(
    wrapper
      .findAll('button')
      .find((button) => button.text() === 'Cancelling…')!
      .attributes('disabled'),
  ).toBeDefined()
  wrapper.unmount()
})