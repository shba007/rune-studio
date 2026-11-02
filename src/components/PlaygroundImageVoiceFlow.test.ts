import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import PlaygroundImageVoiceFlow from './PlaygroundImageVoiceFlow.vue'
import type { ModelCapabilities } from '@/types/protocol'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
  listen: vi.fn<(name: string, handler: (event: unknown) => void) => Promise<() => void>>(),
  handler: null as null | ((event: unknown) => void),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }))
vi.mock('@/config/imageFile', () => ({ readImageAsPng: async () => 'data:image/png;base64,AA==' }))

const models = [
  { id: 'caption', tasks: ['image-to-text'] },
  {
    id: 'voice',
    tasks: ['text-to-audio'],
    audioOperations: { speechGeneration: { status: 'supported' } },
  },
] as ModelCapabilities[]

beforeEach(() => {
  mocks.invoke.mockReset()
  mocks.listen.mockReset()
  mocks.listen.mockImplementation(async (_name: string, handler: (event: unknown) => void) => {
    mocks.handler = handler
    return () => {}
  })
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === 'playground_start_run') return 42
    if (command === 'playground_image_voice_flow')
      return {
        caption: 'A cat',
        speech: {
          audio: 'data:audio/wav;base64,AA==',
          text: 'A cat',
          model: 'voice',
          elapsedMs: 10,
        },
        error: null,
      }
  })
})

async function configured() {
  const wrapper = mount(PlaygroundImageVoiceFlow, {
    props: { models },
    global: { stubs: { teleport: true } },
  })
  await flushPromises()
  const image = wrapper.get('[aria-label="Flow image"]')
  Object.defineProperty(image.element, 'files', {
    configurable: true,
    value: [new File(['x'], 'cat.png', { type: 'image/png' })],
  })
  await image.trigger('change')
  await flushPromises()
  await wrapper.get('button[aria-label="Flow caption model"]').trigger('click')
  await wrapper
    .get('[role="listbox"][aria-label="Flow caption model"]')
    .findAll('[role="option"]')[1]!
    .trigger('click')
  await wrapper.get('button[aria-label="Flow voice model"]').trigger('click')
  await wrapper
    .get('[role="listbox"][aria-label="Flow voice model"]')
    .findAll('[role="option"]')[1]!
    .trigger('click')
  return wrapper
}

it('runs the image to voice flow and displays both outputs', async () => {
  const wrapper = await configured()
  await wrapper.get('[data-testid="flow-run"]').trigger('click')
  await flushPromises()
  expect(mocks.invoke).toHaveBeenCalledWith('playground_image_voice_flow', {
    runId: 42,
    input: {
      image: 'data:image/png;base64,AA==',
      captionModel: 'caption',
      speechModel: 'voice',
      detail: 'concise',
    },
  })
  expect(wrapper.text()).toContain('A cat')
  expect(wrapper.get('audio').attributes('src')).toContain('data:audio/wav')
  expect(wrapper.get('a').attributes('download')).toBe('rune-flow-voice.wav')
  wrapper.unmount()
})

it('keeps the description when speech generation fails', async () => {
  mocks.invoke.mockImplementation(async (command: string) =>
    command === 'playground_start_run'
      ? 42
      : { caption: 'A cat', speech: null, error: 'Speech failed' },
  )
  const wrapper = await configured()
  await wrapper.get('[data-testid="flow-run"]').trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('A cat')
  expect(wrapper.get('[role="alert"]').text()).toContain('Speech failed')
  wrapper.unmount()
})

it('cancels an in-flight run and ignores a late result', async () => {
  let finish!: (value: unknown) => void
  mocks.invoke.mockImplementation(async (command: string) =>
    command === 'playground_start_run'
      ? 42
      : command === 'playground_image_voice_flow'
        ? new Promise((resolve) => {
            finish = resolve
          })
        : undefined,
  )
  const wrapper = await configured()
  await wrapper.get('[data-testid="flow-run"]').trigger('click')
  await flushPromises()
  await wrapper.get('[data-testid="result-cancel"]').trigger('click')
  finish({ caption: 'late', speech: null, error: null })
  await flushPromises()
  expect(mocks.invoke).toHaveBeenCalledWith('playground_cancel_run', { runId: 42 })
  expect(wrapper.text()).not.toContain('late')
  wrapper.unmount()
})

it('shows stage progress and retains the description during speech', async () => {
  let finish!: (value: unknown) => void
  mocks.invoke.mockImplementation(async (command: string) =>
    command === 'playground_start_run'
      ? 42
      : command === 'playground_image_voice_flow'
        ? new Promise((resolve) => {
            finish = resolve
          })
        : undefined,
  )
  const wrapper = await configured()
  await wrapper.get('[data-testid="flow-run"]').trigger('click')
  await flushPromises()
  mocks.handler?.({ payload: { runId: 99, stage: 'speech', caption: 'wrong run' } })
  await flushPromises()
  expect(wrapper.text()).not.toContain('wrong run')
  mocks.handler?.({ payload: { runId: 42, stage: 'speech', caption: 'A cat' } })
  await flushPromises()
  expect(wrapper.text()).toContain('Creating voice')
  expect(wrapper.text()).toContain('A cat')
  finish({ caption: 'A cat', speech: null, error: 'voice unavailable' })
  await flushPromises()
  expect(wrapper.text()).toContain('voice unavailable')
  wrapper.unmount()
})

it('does not allow running without compatible models', async () => {
  const wrapper = mount(PlaygroundImageVoiceFlow, { props: { models: [] } })
  await flushPromises()
  expect(wrapper.get('[data-testid="flow-run"]').attributes('disabled')).toBeDefined()
  expect(wrapper.text()).toContain('verified speech-generation model')
  wrapper.unmount()
})

it('shows the same selected models and result in App and Graph views', async () => {
  const wrapper = await configured()
  const view = wrapper.get('[aria-label="Flow view"]')
  await view.findAll('[role="radio"]')[1]!.trigger('click')
  expect(wrapper.get('[data-testid="flow-graph"]').text()).toContain('caption')
  expect(wrapper.get('[data-testid="flow-graph"]').text()).toContain('voice')
  await wrapper.get('[data-testid="flow-run"]').trigger('click')
  await flushPromises()
  expect(wrapper.get('[data-testid="flow-graph"]').text()).toContain('A cat')
  expect(wrapper.find('[data-testid="flow-graph"] audio').exists()).toBe(true)
  expect(wrapper.get('[data-testid="flow-node-caption"]').text()).toContain('Complete')
  expect(wrapper.get('[data-testid="flow-node-voice"]').text()).toContain('Complete')
  await wrapper
    .get('[aria-label="Description detail"]')
    .findAll('[role="radio"]')[1]!
    .trigger('click')
  expect(wrapper.text()).toContain('output is from the previous run')
  await view.findAll('[role="radio"]')[0]!.trigger('click')
  expect(wrapper.get('[data-testid="flow-app"]').text()).toContain('A cat')
  expect(wrapper.find('[data-testid="flow-app"] audio').exists()).toBe(true)
  wrapper.unmount()
})