import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import { useDecisions } from './useDecisions'
import type { IntakeEntry } from '@/types/decisions'
const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
  event: null as null | (() => void),
  off: vi.fn<() => void>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (_: string, event: () => void) => {
    harness.event = event
    return harness.off
  },
}))
it('ignores stale refreshes and sends only one answer with the saved revision', async () => {
  let initial: ((value: IntakeEntry[]) => void) | undefined
  let finish: (() => void) | undefined
  let reads = 0
  const entry = {
    id: 'q',
    revision: 2,
    text: 'Become fit',
    class: 'health',
    kind: 'goal',
    status: 'clarification',
    question: {
      id: 'q',
      revision: 2,
      kind: 'clarification',
      title: 'Current activity?',
      explanation: '',
      options: [],
    },
    goal: null,
    error: null,
    taskId: null,
  } as IntakeEntry
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'decisions_state') {
      reads++
      return reads === 1
        ? new Promise<IntakeEntry[]>((resolve) => {
            initial = resolve
          })
        : [entry]
    }
    if (command === 'decision_answer')
      return new Promise<void>((resolve) => {
        finish = resolve
      })
    return null
  })
  let decisions: ReturnType<typeof useDecisions> | undefined
  const wrapper = mount(
    defineComponent({
      setup() {
        decisions = useDecisions()
        return () => null
      },
    }),
  )
  await flushPromises()
  harness.event!()
  await flushPromises()
  initial!([])
  await flushPromises()
  expect(decisions!.current.value?.revision).toBe(2)
  const answer = { id: 'q', revision: 2, optionId: null, text: 'I jog twice a week' }
  const first = decisions!.answer(answer)
  await decisions!.answer(answer)
  expect(
    harness.invoke.mock.calls.filter(([command]) => command === 'decision_answer'),
  ).toHaveLength(1)
  finish!()
  await first
  expect(harness.invoke).toHaveBeenCalledWith('decision_answer', answer)
  wrapper.unmount()
  expect(harness.off).toHaveBeenCalledTimes(2)
})