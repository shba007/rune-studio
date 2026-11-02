import { flushPromises, mount } from '@vue/test-utils'
import { computed, ref } from 'vue'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { AnalysisSnapshot, ModelCapabilities } from '@/types/protocol'
import PlaygroundVisualAnalysisFlow from './PlaygroundVisualAnalysisFlow.vue'

const snapshot = ref<AnalysisSnapshot>({ ...EMPTY_SNAPSHOT })
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@/composables/useAnalysis', () => ({
  useAnalysis: () => ({ snapshot, busy: computed(() => false) }),
}))
const models = [
  { id: 'vision-a', imageIn: 'yes', toolCalls: 'yes', tasks: ['text-image-to-text'] },
  { id: 'depth-a', imageIn: 'no', toolCalls: 'no', tasks: ['image-to-depth'] },
] as ModelCapabilities[]
const questions = {
  entities: 'Which objects?',
  regions: 'Which regions?',
  structure: 'Which landmarks?',
  depth: 'What is near?',
}
const picture = 'data:image/png;base64,AA=='
let nextId = 0
beforeEach(() => {
  snapshot.value = { ...EMPTY_SNAPSHOT }
  nextId = 0
  invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'screen_insight_targets')
      return [{ target: { kind: 'screen', id: 1 }, label: 'Main screen', primary: true }]
    if (command === 'playground_start_run') return ++nextId
    if (command === 'playground_visual_plan')
      return { image: picture, model: 'vision-a', questions }
    if (command === 'playground_visual_branch')
      return { answer: 'A red square', findings: [], model: 'vision-a' }
    if (command === 'playground_depth')
      return {
        original: picture,
        preview: null,
        width: 1,
        height: 1,
        min: 0,
        max: 1,
        model: 'depth-a',
        elapsedMs: 50,
      }
  })
})
async function show() {
  const wrapper = mount(PlaygroundVisualAnalysisFlow, {
    props: { models },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  return wrapper
}
async function select(wrapper: Awaited<ReturnType<typeof show>>, name: string) {
  await wrapper.get(`button[aria-label="${name} model"]`).trigger('click')
  await wrapper
    .get(`[role="listbox"][aria-label="${name} model"]`)
    .findAll('[role="option"]')[1]!
    .trigger('click')
}
const runButton = (wrapper: Awaited<ReturnType<typeof show>>) =>
  wrapper.findAll('button').find((button) => button.text() === 'Run Flow')!
async function configure(wrapper: Awaited<ReturnType<typeof show>>) {
  for (const name of [
    'Planning',
    'Entities',
    'Regions & Context',
    'Structure & Landmarks',
    'Depth',
  ])
    await select(wrapper, name)
}

it('plans questions first and launches independent branches against the same frame', async () => {
  const pending: Array<(value: unknown) => void> = []
  invoke.mockImplementation(async (command) => {
    if (command === 'screen_insight_targets')
      return [{ target: { kind: 'screen', id: 1 }, label: 'Main screen', primary: true }]
    if (command === 'playground_start_run') return ++nextId
    if (command === 'playground_visual_plan')
      return { image: picture, model: 'vision-a', questions }
    if (command === 'playground_visual_branch' || command === 'playground_depth')
      return new Promise((resolve) => pending.push(resolve))
  })
  const wrapper = await show()
  await configure(wrapper)
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('playground_visual_plan', {
    runId: 1,
    input: { target: { kind: 'screen', id: 1 }, model: 'vision-a', question: '' },
  })
  expect(pending).toHaveLength(4)
  const branches = invoke.mock.calls.filter(([command]) => command === 'playground_visual_branch')
  expect(branches).toHaveLength(3)
  expect(
    branches.map(
      ([, value]) => (value as { input: { question: string; image: string } }).input.question,
    ),
  ).toEqual([questions.entities, questions.regions, questions.structure])
  expect(
    branches.every(([, value]) => (value as { input: { image: string } }).input.image === picture),
  ).toBe(true)
  const depthCall = invoke.mock.calls.find(([command]) => command === 'playground_depth')![1] as {
    input: { image: string }
  }
  expect(depthCall.input.image).toBe(picture)
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[1]!.trigger('click')
  expect(wrapper.get('[data-testid="visual-flow-graph"]').text()).toContain('7 steps')
  expect(wrapper.get('[data-testid="visual-parallel-branches"]').findAll('article')).toHaveLength(4)
  for (let i = 0; i < 3; i++)
    pending[i]!({ answer: 'A red square', findings: [], model: 'vision-a' })
  pending[3]!({
    original: picture,
    preview: null,
    width: 1,
    height: 1,
    min: 0,
    max: 1,
    model: 'depth-a',
    elapsedMs: 50,
  })
  await flushPromises()
  expect(wrapper.get('[data-testid="visual-node-structure"]').text()).toContain('Complete')
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[0]!.trigger('click')
  expect(wrapper.get('[aria-label="Entities result"]').text()).toContain('A red square')
  expect(wrapper.text()).toContain('Which objects?')
  wrapper.unmount()
})

it('supports optional branches and an optional user question', async () => {
  const wrapper = await show()
  for (const name of ['Regions & Context', 'Structure & Landmarks', 'Depth'])
    await wrapper.get(`[aria-label="Enable ${name}"]`).trigger('click')
  await select(wrapper, 'Planning')
  await select(wrapper, 'Entities')
  await wrapper.get('#visual-flow-question').setValue('Find the window')
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('playground_visual_plan', {
    runId: 1,
    input: { target: { kind: 'screen', id: 1 }, model: 'vision-a', question: 'Find the window' },
  })
  expect(
    invoke.mock.calls.filter(([command]) => command === 'playground_visual_branch'),
  ).toHaveLength(1)
  expect(invoke.mock.calls.filter(([command]) => command === 'playground_depth')).toHaveLength(0)
  wrapper.unmount()
})

it('isolates a failed branch and retains successful siblings', async () => {
  invoke.mockImplementation(async (command, args) => {
    if (command === 'screen_insight_targets')
      return [{ target: { kind: 'screen', id: 1 }, label: 'Main screen', primary: true }]
    if (command === 'playground_start_run') return ++nextId
    if (command === 'playground_visual_plan')
      return { image: picture, model: 'vision-a', questions }
    if (command === 'playground_visual_branch') {
      if ((args as { input: { question: string } }).input.question === questions.regions)
        throw new Error('Model unavailable')
      return { answer: 'Done', findings: [], model: 'vision-a' }
    }
    if (command === 'playground_depth')
      return {
        original: picture,
        preview: null,
        width: 1,
        height: 1,
        min: 0,
        max: 1,
        model: 'depth-a',
        elapsedMs: 50,
      }
  })
  const wrapper = await show()
  await configure(wrapper)
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(wrapper.get('[aria-label="Regions & Context result"]').text()).toContain(
    'Model unavailable',
  )
  expect(wrapper.get('[aria-label="Structure & Landmarks result"]').text()).toContain('Complete')
  expect(wrapper.get('[aria-label="Depth result"]').text()).toContain('Complete')
  wrapper.unmount()
})

it('stops before any branch when the planner fails', async () => {
  const base = invoke.getMockImplementation()!
  invoke.mockImplementation(async (command, args) =>
    command === 'playground_visual_plan'
      ? Promise.reject(new Error('Planner unavailable'))
      : base(command, args),
  )
  const wrapper = await show()
  await configure(wrapper)
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Planner unavailable')
  expect(
    invoke.mock.calls.filter(
      ([command]) => command === 'playground_visual_branch' || command === 'playground_depth',
    ),
  ).toHaveLength(0)
  wrapper.unmount()
})

it('cancels all active branches and keeps their results cancelled', async () => {
  const pending: Array<(value: unknown) => void> = []
  const base = invoke.getMockImplementation()!
  invoke.mockImplementation(async (command, args) => {
    if (command === 'playground_visual_branch' || command === 'playground_depth')
      return new Promise((resolve) => pending.push(resolve))
    return base(command, args)
  })
  const wrapper = await show()
  await configure(wrapper)
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(pending).toHaveLength(4)
  await wrapper.get('[data-testid="result-cancel"]').trigger('click')
  expect(invoke.mock.calls.filter(([command]) => command === 'playground_cancel_run')).toHaveLength(
    4,
  )
  for (const finish of pending) finish({ answer: 'late', findings: [], model: 'vision-a' })
  await flushPromises()
  expect(wrapper.text()).toContain('Flow cancelled')
  expect(wrapper.get('[aria-label="Entities result"]').text()).toContain('Cancelled')
  expect(wrapper.text()).not.toContain('late')
  wrapper.unmount()
})