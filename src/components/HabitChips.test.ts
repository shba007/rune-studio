import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'

import { useHabits } from '@/composables/useHabits'
import type { Habit } from '@/config/habits'

import HabitChips from './HabitChips.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<() => Promise<() => void>>(async () => () => {}),
}))

const base = (patch: Partial<Habit>): Habit => ({
  id: 'h',
  title: 'Habit',
  targetMinutes: null,
  state: 'pending',
  dueMs: Date.now() - 60_000,
  completedMs: null,
  creditedMs: 0,
  ...patch,
})

async function show(habits: Habit[], failWith?: { command: string; message: string }) {
  api.invoke.mockReset().mockImplementation(async (command) => {
    if (failWith && failWith.command === command) throw failWith.message
    return command === 'habits_state' ? habits : null
  })
  const wrapper = mount({
    components: { HabitChips },
    setup: () => ({ habits: useHabits() }),
    template: '<HabitChips :habits="habits" />',
  })
  await flushPromises()
  return wrapper
}
const calls = (name: string) => api.invoke.mock.calls.filter(([command]) => command === name)

beforeEach(() => api.invoke.mockReset())

it('checks a plain habit in with one click', async () => {
  const wrapper = await show([base({ id: 'water', title: 'Drink water' })])
  await wrapper.get('button[aria-label="Check in Drink water"]').trigger('click')
  await flushPromises()
  expect(calls('habit_check_in')[0]![1]).toEqual({ id: 'water' })
})

it('starts a focus block for a timed habit and shows its progress', async () => {
  const wrapper = await show([
    base({ id: 'read', title: 'Read', targetMinutes: 30, creditedMs: 10 * 60_000 }),
  ])
  const chip = wrapper.get('button[aria-label="Start focus for Read"]')
  expect(chip.text()).toContain('Read · 10/30m')
  await chip.trigger('click')
  await flushPromises()
  expect(calls('focus_start_task')[0]![1]).toEqual({ taskId: 'read' })
  expect(calls('habit_check_in')).toHaveLength(0)
})

it('shows a habit done today as checked and not clickable', async () => {
  const wrapper = await show([
    base({ id: 'water', title: 'Drink water', state: 'completed', completedMs: Date.now() - 1000 }),
  ])
  const chip = wrapper.get('button[aria-label="Drink water: done today"]')
  expect(chip.attributes('disabled')).toBeDefined()
  expect(chip.attributes('aria-pressed')).toBe('true')
})

it('shows why a check-in failed and keeps the chip', async () => {
  const wrapper = await show([base({ id: 'water', title: 'Drink water' })], {
    command: 'habit_check_in',
    message: 'This habit is already checked in for today.',
  })
  await wrapper.get('button[aria-label="Check in Drink water"]').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role=alert]').text()).toContain('already checked in')
  expect(wrapper.find('button[aria-label="Check in Drink water"]').exists()).toBe(true)
})

it('reads the habits again after an action', async () => {
  const wrapper = await show([base({ id: 'water', title: 'Drink water' })])
  const before = calls('habits_state').length
  await wrapper.get('button[aria-label="Check in Drink water"]').trigger('click')
  await flushPromises()
  expect(calls('habits_state').length).toBe(before + 1)
})