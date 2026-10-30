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
it('requires explicit confirmation before a scheduled Rune task runs automatically', async () => {
  const wrapper = mount(NewWorkForm, { props: { goals: [] } })
  await flushPromises()
  await wrapper.get('textarea').setValue('Configure app alarms')
  await wrapper.get('button[aria-label="Who will do this task?"]').trigger('click')
  await wrapper
    .findAll('[role="option"]')
    .find((option) => option.text().startsWith('Rune'))!
    .trigger('click')
  expect(wrapper.text()).not.toContain('When due')
  await wrapper.get('[role="switch"]').trigger('click')
  await wrapper.get('input[type="datetime-local"]').setValue('2027-01-31T07:00')
  expect(wrapper.text()).toContain('When due')
  await wrapper
    .findAll('[role="radio"]')
    .find((button) => button.text() === 'Run automatically')!
    .trigger('click')
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[role="switch"][aria-label="Confirm automatic running"]').trigger('click')
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeUndefined()
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith(
    'workspace_task_create',
    expect.objectContaining({ executionMode: 'agent', dueAction: 'run' }),
  )
})
it('never offers automatic running for manual work', async () => {
  const wrapper = mount(NewWorkForm, { props: { goals: [] } })
  await flushPromises()
  await wrapper.get('textarea').setValue('Go jogging')
  await wrapper.get('[role="switch"]').trigger('click')
  await wrapper.get('input[type="datetime-local"]').setValue('2027-01-31T07:00')
  expect(wrapper.text()).not.toContain('Run automatically')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith(
    'workspace_task_create',
    expect.objectContaining({ dueAction: 'remind' }),
  )
})