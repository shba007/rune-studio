import { flushPromises, mount } from '@vue/test-utils'
import { computed, ref } from 'vue'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { AnalysisSnapshot, ModelCapabilities, VisualAnalysisModels } from '@/types/protocol'
import PlaygroundVisualAnalysisFlow from './PlaygroundVisualAnalysisFlow.vue'

const snapshot = ref<AnalysisSnapshot>({ ...EMPTY_SNAPSHOT })
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@/composables/useAnalysis', () => ({
  useAnalysis: () => ({ snapshot, busy: computed(() => false) }),
}))
const models = [
  { id: 'vision-a', imageIn: 'yes', toolCalls: 'yes', tasks: ['text-image-to-text'] },
  {
    id: 'detect-a',
    imageIn: 'no',
    toolCalls: 'no',
    tasks: ['image-to-detection'],
    rawCapabilities: [],
  },
  {
    id: 'pose-a',
    imageIn: 'no',
    toolCalls: 'no',
    tasks: ['image-to-detection'],
    rawCapabilities: ['Pose Estimation'],
  },
  {
    id: 'segment-a',
    imageIn: 'no',
    toolCalls: 'no',
    tasks: ['image-to-segmentation'],
    rawCapabilities: [],
  },
  { id: 'depth-a', imageIn: 'no', toolCalls: 'no', tasks: ['image-to-depth'] },
  {
    id: 'mask-a',
    imageIn: 'no',
    toolCalls: 'no',
    tasks: ['image-to-detection'],
    rawCapabilities: ['Instance Segmentation'],
  },
  {
    id: 'oriented-a',
    imageIn: 'no',
    toolCalls: 'no',
    tasks: ['image-to-detection'],
    rawCapabilities: ['Oriented Bounding Boxes'],
  },
] as ModelCapabilities[]
const questions = {
  entities: 'Which objects?',
  regions: 'Which regions?',
  structure: 'Which landmarks?',
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
      return { result: { model: 'detect-a', data: [{ label: 'A red square', confidence: 0.9 }] } }
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

it('uses the shared Visual Analysis settings and allows editing before a run', async () => {
  const wrapper = mount(PlaygroundVisualAnalysisFlow, {
    props: {
      models,
      visualModels: {
        planner: 'vision-a',
        entities: 'detect-a',
        regions: '',
        structure: '',
        depth: '',
      },
    },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  expect(wrapper.get('button[aria-label="Planning model"]').text()).toContain('vision-a')
  expect(wrapper.get('button[aria-label="Entities model"]').text()).toContain('detect-a')
  expect(wrapper.get('button[aria-label="Planning model"]').attributes('disabled')).toBeUndefined()
  expect(
    wrapper.get('button[aria-label="Enable Regions & Context"]').attributes('aria-checked'),
  ).toBe('false')
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith(
    'playground_visual_branch',
    expect.objectContaining({
      input: { image: picture, model: 'detect-a', role: 'entities' },
    }),
  )
  wrapper.unmount()
})

it('requires shared model changes to be applied before running', async () => {
  const save = vi.fn<(models: VisualAnalysisModels) => Promise<boolean>>().mockResolvedValue(true)
  const shared = {
    planner: 'vision-a',
    entities: 'detect-a',
    regions: '',
    structure: '',
    depth: '',
  }
  const wrapper = mount(PlaygroundVisualAnalysisFlow, {
    props: { models, visualModels: shared, saveVisualModels: save },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  await wrapper.get('button[aria-label="Enable Regions & Context"]').trigger('click')
  await select(wrapper, 'Regions & Context')
  expect(runButton(wrapper).attributes('disabled')).toBeDefined()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Apply shared models')!
    .trigger('click')
  await flushPromises()
  expect(save).toHaveBeenCalledWith(
    expect.objectContaining({
      planner: 'vision-a',
      entities: 'detect-a',
      regions: expect.any(String),
    }),
  )
  expect(runButton(wrapper).attributes('disabled')).toBeDefined()
  await wrapper.setProps({ visualModels: { ...shared, regions: save.mock.calls[0]![0].regions } })
  expect(runButton(wrapper).attributes('disabled')).toBeUndefined()
  wrapper.unmount()
})

it('does not run when applying shared model changes fails', async () => {
  const save = vi.fn<(models: VisualAnalysisModels) => Promise<boolean>>().mockResolvedValue(false)
  const wrapper = mount(PlaygroundVisualAnalysisFlow, {
    props: {
      models,
      visualModels: {
        planner: 'vision-a',
        entities: 'detect-a',
        regions: '',
        structure: '',
        depth: '',
      },
      saveVisualModels: save,
    },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  await wrapper.get('button[aria-label="Enable Regions & Context"]').trigger('click')
  await select(wrapper, 'Regions & Context')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Apply shared models')!
    .trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Could not save shared models')
  expect(runButton(wrapper).attributes('disabled')).toBeDefined()
  wrapper.unmount()
})

it('runs instance masks and oriented objects as separate Flow branches', async () => {
  const wrapper = mount(PlaygroundVisualAnalysisFlow, {
    props: {
      models,
      visualModels: {
        planner: 'vision-a',
        entities: '',
        regions: '',
        structure: '',
        depth: '',
        instances: 'mask-a',
        oriented: 'oriented-a',
      },
    },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  await runButton(wrapper).trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith(
    'playground_visual_branch',
    expect.objectContaining({ input: { image: picture, model: 'mask-a', role: 'instances' } }),
  )
  expect(invoke).toHaveBeenCalledWith(
    'playground_visual_branch',
    expect.objectContaining({ input: { image: picture, model: 'oriented-a', role: 'oriented' } }),
  )
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[1]!.trigger('click')
  expect(wrapper.get('[data-testid="visual-parallel-branches"]').text()).toContain(
    'Instance segmentation',
  )
  expect(wrapper.get('[data-testid="visual-parallel-branches"]').text()).toContain(
    'Oriented detection',
  )
  wrapper.unmount()
})

it('shows every configured model in the graph in settings order', async () => {
  const wrapper = mount(PlaygroundVisualAnalysisFlow, {
    props: {
      models,
      visualModels: {
        planner: 'vision-a',
        entities: 'detect-a',
        instances: 'mask-a',
        oriented: 'oriented-a',
        regions: 'segment-a',
        structure: 'pose-a',
        depth: 'depth-a',
      },
    },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[1]!.trigger('click')
  const nodes = wrapper.get('[data-testid="visual-parallel-branches"]').findAll('article')
  expect(nodes).toHaveLength(6)
  for (const [index, [label, model]] of [
    ['Detection', 'detect-a'],
    ['Instance segmentation', 'mask-a'],
    ['Oriented detection', 'oriented-a'],
    ['Segmentation', 'segment-a'],
    ['Pose', 'pose-a'],
    ['Depth', 'depth-a'],
  ].entries()) {
    expect(nodes[index]!.text()).toContain(label)
    expect(nodes[index]!.text()).toContain(model)
  }
  expect(wrapper.get('[data-testid="visual-flow-graph"]').text()).toContain('9 steps')
  expect(wrapper.get('[data-testid="visual-flow-graph"]').attributes('style')).toContain('2016px')
  const fit = wrapper.get('[aria-label="Fit graph"]')
  const initialZoom = fit.text()
  await wrapper.get('[aria-label="Zoom in"]').trigger('click')
  expect(fit.text()).not.toBe(initialZoom)
  await fit.trigger('click')
  expect(fit.text()).toBe(initialZoom)
  wrapper
    .get('[data-testid="visual-flow-graph"]')
    .element.parentElement!.parentElement!.dispatchEvent(
      new WheelEvent('wheel', { ctrlKey: true, deltaY: 1, bubbles: true, cancelable: true }),
    )
  await wrapper.vm.$nextTick()
  expect(fit.text()).not.toBe(initialZoom)
  wrapper.unmount()
})

it('keeps a selected target on refresh and chooses the new primary when it disappears', async () => {
  const wrapper = await show()
  const picker = wrapper.get('[aria-label="Choose capture target"]')
  expect(picker.text()).toContain('Main screen')
  invoke.mockResolvedValueOnce([
    { target: { kind: 'screen', id: 1 }, label: 'Main screen', primary: true },
    { target: { kind: 'window', id: 2 }, label: 'Editor' },
  ])
  await wrapper.get('[aria-label="Refresh capture targets"]').trigger('click')
  await flushPromises()
  await picker.trigger('click')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Editor')!
    .trigger('click')
  invoke.mockResolvedValueOnce([
    { target: { kind: 'screen', id: 1 }, label: 'Main screen', primary: true },
    { target: { kind: 'window', id: 2 }, label: 'Editor' },
  ])
  await wrapper.get('[aria-label="Refresh capture targets"]').trigger('click')
  await flushPromises()
  expect(picker.text()).toContain('Editor')
  invoke.mockResolvedValueOnce([
    { target: { kind: 'screen', id: 3 }, label: 'New screen', primary: true },
  ])
  await wrapper.get('[aria-label="Refresh capture targets"]').trigger('click')
  await flushPromises()
  expect(picker.text()).toContain('New screen')
  wrapper.unmount()
})

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
    branches.map(([, value]) => (value as { input: { role: string; image: string } }).input.role),
  ).toEqual(['entities', 'regions', 'structure'])
  expect(
    branches.every(([, value]) => (value as { input: { image: string } }).input.image === picture),
  ).toBe(true)
  const depthCall = invoke.mock.calls.find(([command]) => command === 'playground_depth')![1] as {
    input: { image: string }
  }
  expect(depthCall.input.image).toBe(picture)
  expect(depthCall.input).not.toHaveProperty('question')
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[1]!.trigger('click')
  expect(
    wrapper.get('[data-testid="graph-progress"]').find('[data-testid="result-cancel"]').exists(),
  ).toBe(true)
  expect(wrapper.get('[data-testid="visual-flow-graph"]').text()).toContain('7 steps')
  expect(wrapper.get('[data-testid="visual-parallel-branches"]').findAll('article')).toHaveLength(4)
  for (let i = 0; i < 3; i++)
    pending[i]!({
      result: { data: [{ label: 'A red square', confidence: 0.9 }], model: 'detect-a' },
    })
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
  expect(wrapper.get('[data-testid="visual-node-depth"]').text()).not.toContain('Question')
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[0]!.trigger('click')
  expect(wrapper.get('[aria-label="Entities result"]').text()).toContain('A red square')
  expect(wrapper.text()).toContain('Which objects?')
  await wrapper.get('[role="tab"][aria-label="Entities"]').trigger('keydown', { key: 'ArrowRight' })
  expect(
    wrapper.get('[role="tab"][aria-label="Regions & Context"]').attributes('aria-selected'),
  ).toBe('true')
  await wrapper.get('[role="tab"][aria-label="Depth"]').trigger('click')
  expect(wrapper.get('[aria-label="Depth result"]').text()).toContain('depth-a')
  expect(wrapper.get('[aria-label="Depth result"]').text()).not.toContain('Question')
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

it('shows semantic classes and pose keypoints returned by Infer', async () => {
  const base = invoke.getMockImplementation()!
  invoke.mockImplementation(async (command, args) => {
    if (command === 'playground_visual_branch') {
      const role = (args as { input: { role: string } }).input.role
      if (role === 'regions')
        return {
          result: { model: 'segment-a', class_map: picture, classes: [{ id: 3, label: 'wall' }] },
        }
      if (role === 'structure')
        return {
          result: {
            model: 'pose-a',
            data: [
              {
                label: 'person',
                confidence: 0.9,
                pose: [{ name: 'nose', x: 0.5, y: 0.4, confidence: 0.8 }],
              },
            ],
          },
        }
    }
    return base(command, args)
  })
  const wrapper = await show()
  await configure(wrapper)
  await runButton(wrapper).trigger('click')
  await flushPromises()
  await wrapper.get('[role="tab"][aria-label="Regions & Context"]').trigger('click')
  expect(wrapper.find('[aria-label="Colored semantic segmentation"]').exists()).toBe(true)
  expect(wrapper.get('[aria-label="Regions & Context result"]').text()).toContain('wall (3)')
  await wrapper.get('[role="tab"][aria-label="Structure & Landmarks"]').trigger('click')
  expect(wrapper.get('[data-keypoint="nose"]').attributes('style')).toContain('50%')
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
      if ((args as { input: { role: string } }).input.role === 'regions')
        throw new Error('Model unavailable')
      return { result: { data: [], model: 'detect-a' } }
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
  await wrapper.get('[role="tab"][aria-label="Regions & Context"]').trigger('click')
  expect(wrapper.get('[aria-label="Regions & Context result"]').text()).toContain(
    'Model unavailable',
  )
  await wrapper.get('[role="tab"][aria-label="Structure & Landmarks"]').trigger('click')
  expect(wrapper.get('[aria-label="Structure & Landmarks result"]').text()).toContain('Complete')
  await wrapper.get('[role="tab"][aria-label="Depth"]').trigger('click')
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
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[1]!.trigger('click')
  await wrapper.get('[data-testid="graph-progress"] [data-testid="result-cancel"]').trigger('click')
  expect(invoke.mock.calls.filter(([command]) => command === 'playground_cancel_run')).toHaveLength(
    4,
  )
  for (const finish of pending) finish({ answer: 'late', findings: [], model: 'vision-a' })
  await flushPromises()
  expect(wrapper.text()).toContain('Flow cancelled')
  await wrapper.get('[aria-label="Flow view"]').findAll('[role="radio"]')[0]!.trigger('click')
  expect(wrapper.get('[aria-label="Entities result"]').text()).toContain('Cancelled')
  expect(wrapper.text()).not.toContain('late')
  wrapper.unmount()
})