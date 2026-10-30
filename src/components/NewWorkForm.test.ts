import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import NewWorkForm from './NewWorkForm.vue'
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<string>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
beforeEach(() => {
  invoke
    .mockReset()
    .mockImplementation((command) =>
      Promise.resolve(command === 'workspace_task_zone' ? 'UTC' : 'created-id'),
    )
})
it('saves manual work without dispatching execution', async () => {
  const wrapper = mount(NewWorkForm, { props: { goals: [] } })
  await flushPromises()
  await wrapper.get('textarea').setValue('Go jogging')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith(
    'workspace_task_create',
    expect.objectContaining({ title: 'Go jogging', executionMode: 'manual', schedule: null }),
  )
  expect(invoke.mock.calls.some(([command]) => command === 'workspace_task_action')).toBe(false)
  expect(wrapper.emitted('saved')).toEqual([['task', 'created-id']])
})
it('saves a monthly Visual Analysis reminder with its original monthly day', async () => {
  const wrapper = mount(NewWorkForm, { props: { goals: [] } })
  await flushPromises()
  await wrapper.get('textarea').setValue('Inspect this screen')
  await wrapper.get('button[aria-label="Who will do this task?"]').trigger('click')
  await wrapper
    .findAll('[role="option"]')
    .find((option) => option.text().startsWith('Visual Analysis'))!
    .trigger('click')
  await wrapper.get('[role="switch"]').trigger('click')
  await wrapper.get('input[type="datetime-local"]').setValue('2027-01-31T07:00')
  await wrapper
    .findAll('[role="radio"]')
    .find((button) => button.text() === 'Monthly')!
    .trigger('click')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith(
    'workspace_task_create',
    expect.objectContaining({
      executionMode: 'analysis',
      schedule: { local: '2027-01-31T07:00', zone: 'UTC', repeat: 'monthly', anchorDay: 31 },
    }),
  )
})