import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'

import { DEFAULT_PRESET, EMPTY_FOCUS, type FocusPreset } from '@/config/focus'

import SettingsFocus from './SettingsFocus.vue'

const invoke = vi.hoisted(() =>
  vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
)
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

const student: FocusPreset = {
  ...DEFAULT_PRESET,
  id: 'student',
  label: 'Student',
  summary: 'Shorter blocks.',
  capMinutes: 50,
  bufferMinutes: 10,
}
const snapshot = (preset: FocusPreset) => ({
  ...EMPTY_FOCUS,
  preset,
  presets: [DEFAULT_PRESET, student],
})

beforeEach(() => {
  invoke.mockReset().mockImplementation(async (command, args) => {
    if (command === 'focus_preset_set')
      return snapshot(args?.id === 'student' ? student : DEFAULT_PRESET)
    return snapshot(DEFAULT_PRESET)
  })
})

const pressed = (wrapper: ReturnType<typeof mount>) =>
  wrapper.findAll('button[aria-pressed="true"]').map((b) => b.text())

it('lists the presets and marks the one in use', async () => {
  const wrapper = mount(SettingsFocus)
  await flushPromises()
  expect(wrapper.findAll('button').map((b) => b.text())).toEqual(['Engineer', 'Student'])
  expect(pressed(wrapper)).toEqual(['Engineer'])
  expect(wrapper.text()).toContain('Blocks up to 120 min')
})

it('switches the preset through the backend and shows its numbers', async () => {
  const wrapper = mount(SettingsFocus)
  await flushPromises()
  await wrapper.findAll('button')[1]!.trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('focus_preset_set', { id: 'student' })
  expect(pressed(wrapper)).toEqual(['Student'])
  expect(wrapper.text()).toContain('Blocks up to 50 min, 10 min kept clear')
})

it('does nothing when the preset in use is chosen again', async () => {
  const wrapper = mount(SettingsFocus)
  await flushPromises()
  await wrapper.findAll('button')[0]!.trigger('click')
  expect(invoke).not.toHaveBeenCalledWith('focus_preset_set', expect.anything())
})

it('shows why the preset could not change and keeps the old one', async () => {
  const wrapper = mount(SettingsFocus)
  await flushPromises()
  invoke.mockImplementation(async (command) => {
    if (command === 'focus_preset_set') throw 'Unknown focus preset "student".'
    return snapshot(DEFAULT_PRESET)
  })
  await wrapper.findAll('button')[1]!.trigger('click')
  await flushPromises()
  expect(wrapper.get('[role=alert]').text()).toContain('Unknown focus preset')
  expect(pressed(wrapper)).toEqual(['Engineer'])
})