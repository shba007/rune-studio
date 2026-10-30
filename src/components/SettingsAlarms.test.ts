import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import SettingsAlarms from './SettingsAlarms.vue'

const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<boolean>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

beforeEach(() => {
  localStorage.clear()
  invoke
    .mockReset()
    .mockImplementation((command, args) =>
      Promise.resolve(
        command === 'alarms_set_autostart' ? (args as { enabled: boolean }).enabled : false,
      ),
    )
})

it('toggles the reminder sound locally and start-with-Windows through the backend', async () => {
  const wrapper = mount(SettingsAlarms)
  await flushPromises()
  const [sound, autostart] = wrapper.findAll('[role="switch"]')
  expect(sound!.attributes('aria-checked')).toBe('true')
  expect(autostart!.attributes('aria-checked')).toBe('false')
  await sound!.trigger('click')
  expect(localStorage.getItem('rune.reminderSound')).toBe('false')
  await autostart!.trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('alarms_set_autostart', { enabled: true })
  expect(wrapper.findAll('[role="switch"]')[1]!.attributes('aria-checked')).toBe('true')
  wrapper.unmount()
})

it('shows why start-with-Windows could not change', async () => {
  invoke.mockImplementation((command) =>
    command === 'alarms_autostart' ? Promise.resolve(false) : Promise.reject(new Error('denied')),
  )
  const wrapper = mount(SettingsAlarms)
  await flushPromises()
  await wrapper.findAll('[role="switch"]')[1]!.trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('denied')
  wrapper.unmount()
})