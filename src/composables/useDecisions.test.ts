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
it('treats domain inference as busy and domain confirmation as a resumable user decision', async () => {
  harness.invoke.mockReset()
  let entry = {
    id: 'domain-goal',
    revision: 1,
    kind: 'goal',
    status: 'domain_planning',
    question: null,
  } as IntakeEntry
  harness.invoke.mockImplementation(async (command) =>
    command === 'decisions_state' ? [entry] : null,
  )
  let decisions!: ReturnType<typeof useDecisions>
  const wrapper = mount(
    defineComponent({
      setup() {
        decisions = useDecisions()
        return () => null
      },
    }),
  )
  await flushPromises()
  expect(decisions.busy.value).toBe(true)
  entry = {
    ...entry,
    revision: 2,
    status: 'domain_selection',
    question: {
      id: entry.id,
      revision: 2,
      kind: 'domain',
      title: 'Which domain?',
      explanation: '',
      options: [],
    },
  }
  await decisions.refresh()
  expect(decisions.busy.value).toBe(false)
  expect(decisions.current.value?.question?.kind).toBe('domain')
  await decisions.answer({ id: entry.id, revision: 2, optionId: null, text: 'family' })
  expect(harness.invoke).toHaveBeenCalledWith('decision_answer', {
    id: entry.id,
    revision: 2,
    optionId: null,
    text: 'family',
  })
  wrapper.unmount()
})

it('treats sub-domain planning as busy and its pending questions as user decisions', async () => {
  harness.invoke.mockReset()
  let entry = {
    id: 'parts',
    revision: 1,
    kind: 'goal',
    status: 'subdomain_planning',
    question: null,
  } as IntakeEntry
  harness.invoke.mockImplementation(async (command) =>
    command === 'decisions_state' ? [entry] : null,
  )
  let decisions!: ReturnType<typeof useDecisions>
  const wrapper = mount(
    defineComponent({
      setup() {
        decisions = useDecisions()
        return () => null
      },
    }),
  )
  await flushPromises()
  expect(decisions.busy.value).toBe(true)
  entry = {
    ...entry,
    revision: 2,
    status: 'subdomain_detail',
    question: {
      id: entry.id,
      revision: 2,
      kind: 'subdomain_detail',
      domain: 'environment',
      title: 'What interrupts you?',
      explanation: '',
      options: [],
    },
  }
  await decisions.refresh()
  expect(decisions.busy.value).toBe(false)
  await decisions.answer({ id: entry.id, revision: 2, optionId: 'skip_all', text: '' })
  expect(harness.invoke).toHaveBeenCalledWith('decision_answer', {
    id: entry.id,
    revision: 2,
    optionId: 'skip_all',
    text: '',
  })
  wrapper.unmount()
})

it('treats Auto inference as busy and its discovery questions as resumable decisions', async () => {
  harness.invoke.mockReset()
  let entry = {
    id: 'auto-goal',
    revision: 1,
    kind: 'goal',
    domainMode: 'auto',
    status: 'discovery_planning',
    text: '',
    question: null,
  } as IntakeEntry
  harness.invoke.mockImplementation(async (command) =>
    command === 'decisions_state' ? [entry] : null,
  )
  let decisions!: ReturnType<typeof useDecisions>
  const wrapper = mount(
    defineComponent({
      setup() {
        decisions = useDecisions()
        return () => null
      },
    }),
  )
  await flushPromises()
  expect(decisions.busy.value).toBe(true)
  entry = {
    ...entry,
    revision: 2,
    status: 'discovery',
    question: {
      id: entry.id,
      revision: 2,
      kind: 'discovery',
      domain: 'physical',
      title: 'How rested do you feel?',
      explanation: '',
      options: [],
    },
  }
  await decisions.refresh()
  expect(decisions.busy.value).toBe(false)
  expect(decisions.current.value?.question?.kind).toBe('discovery')
  await decisions.answer({ id: entry.id, revision: 2, optionId: null, text: 'It varies' })
  expect(harness.invoke).toHaveBeenCalledWith('decision_answer', {
    id: entry.id,
    revision: 2,
    optionId: null,
    text: 'It varies',
  })
  wrapper.unmount()
})

it('versions pathway requests and selection without dispatching executable tasks', async () => {
  harness.invoke.mockReset()
  const entry = {
    id: 'goal',
    revision: 9,
    kind: 'goal',
    status: 'goal_ready',
    question: null,
  } as IntakeEntry
  harness.invoke.mockImplementation(async (command) =>
    command === 'decisions_state' ? [entry] : null,
  )
  let decisions!: ReturnType<typeof useDecisions>
  const wrapper = mount(
    defineComponent({
      setup() {
        decisions = useDecisions()
        return () => null
      },
    }),
  )
  await flushPromises()
  await decisions.comparePathways(entry)
  expect(harness.invoke).toHaveBeenCalledWith('decision_pathways', { id: 'goal', revision: 9 })
  await decisions.selectPathway(entry, 'route-1')
  expect(harness.invoke).toHaveBeenCalledWith('decision_select_pathway', {
    id: 'goal',
    revision: 9,
    pathwayId: 'route-1',
  })
  expect(harness.invoke.mock.calls.some(([command]) => /submit|start_task/.test(command))).toBe(
    false,
  )
  wrapper.unmount()
})
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
it.each([false, true])(
  'refreshes deletion and preserves a failure after refresh: %s',
  async (fails) => {
    harness.invoke.mockReset()
    const entry = { id: 'goal-delete', revision: 7, kind: 'goal' } as IntakeEntry
    let entries = [entry]
    harness.invoke.mockImplementation(async (command) => {
      if (command === 'decisions_state') return entries
      if (command === 'decision_delete') {
        if (fails) throw new Error('Review the latest goal before deleting it')
        entries = []
      }
      return null
    })
    let decisions!: ReturnType<typeof useDecisions>
    const wrapper = mount(
      defineComponent({
        setup() {
          decisions = useDecisions()
          return () => null
        },
      }),
    )
    await flushPromises()
    expect(await decisions.remove(entry)).toBe(!fails)
    expect(harness.invoke).toHaveBeenCalledWith('decision_delete', { id: entry.id, revision: 7 })
    expect(decisions.entries.value).toHaveLength(fails ? 1 : 0)
    expect(decisions.error.value).toBe(fails ? 'Review the latest goal before deleting it' : '')
    expect(decisions.pending.value).toBe(false)
    wrapper.unmount()
  },
)

it('versions task proposal approval separately and preserves conflicts after refresh', async () => {
  harness.invoke.mockReset()
  const entry = {
    id: 'goal',
    revision: 12,
    status: 'goal_ready',
    kind: 'goal',
    question: null,
  } as IntakeEntry
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'decisions_state') return [entry]
    if (command === 'decision_review_task') throw new Error('Conflict: the pathway changed')
    return null
  })
  let decisions!: ReturnType<typeof useDecisions>
  const wrapper = mount(
    defineComponent({
      setup() {
        decisions = useDecisions()
        return () => null
      },
    }),
  )
  await flushPromises()
  await decisions.proposeTasks(entry)
  expect(harness.invoke).toHaveBeenCalledWith('decision_propose_tasks', {
    id: 'goal',
    revision: 12,
  })
  const approval = {
    title: 'Walk',
    instructions: 'Walk for the agreed duration',
    executionMode: 'manual' as const,
    schedule: null,
  }
  await decisions.reviewTask(entry, 'task-1', approval)
  expect(harness.invoke).toHaveBeenCalledWith('decision_review_task', {
    id: 'goal',
    revision: 12,
    proposalId: 'task-1',
    approval,
  })
  expect(decisions.error.value).toContain('Conflict')
  expect(harness.invoke.mock.calls.some(([command]) => command === 'workspace_task_action')).toBe(
    false,
  )
  wrapper.unmount()
})