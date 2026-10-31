import { flushPromises, mount } from '@vue/test-utils'
import { computed, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'

import PlaygroundWindow from './PlaygroundWindow.vue'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import { usePlaygroundHistory } from '@/composables/usePlaygroundHistory'
import type { AnalysisSnapshot } from '@/types/protocol'

const snapshot = ref<AnalysisSnapshot>({ ...EMPTY_SNAPSHOT })
const capture = vi.fn<(...args: unknown[]) => Promise<boolean>>(async () => true)
const cancel = vi.fn<() => Promise<boolean>>(async () => true)
const captureImage = vi.fn<(...args: unknown[]) => Promise<boolean>>(async () => true)

const invoked = vi.fn<(command: string, args?: unknown) => Promise<unknown>>()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: async (command: string, args?: unknown) => {
    void invoked(command, args)
    if (command === 'playground_create')
      return {
        image: 'data:image/png;base64,AA==',
        prompt: 'a red fox',
        size: '512x512',
        model: 'img-m',
        elapsedMs: 4000,
      }
    if (command === 'playground_upscale')
      return {
        original: 'data:image/jpeg;base64,AA==',
        output: 'data:image/png;base64,AA==',
        inputWidth: 2,
        inputHeight: 2,
        outputWidth: 8,
        outputHeight: 8,
        scale: 4,
        model: 'up-x',
        elapsedMs: 900,
      }
    if (command === 'playground_depth')
      return {
        original: 'data:image/jpeg;base64,AA==',
        preview: 'data:image/png;base64,AA==',
        width: 2,
        height: 2,
        min: 0,
        max: 1,
        model: 'depth-s',
        elapsedMs: 1200,
      }
    return command === 'screen_insight_targets'
      ? [
          { target: { kind: 'screen', id: 1 }, label: 'Screen 1', primary: true },
          { target: { kind: 'window', id: 7 }, label: 'Notepad' },
        ]
      : null
  },
}))
vi.mock('@/config/imageFile', () => ({
  readImageAsPng: async () => 'data:image/png;base64,AA==',
}))
vi.mock('@/composables/useNativeBlur', () => ({ useNativeBlur: () => {} }))
vi.mock('@/composables/useAnalysis', () => ({
  useAnalysis: () => ({
    snapshot,
    busy: computed(() => ['capturing', 'asking'].includes(snapshot.value.status)),
    capture,
    captureImage,
    cancel,
  }),
}))
vi.mock('@/composables/useAgent', () => ({
  useAgent: () => ({
    settings: ref({ models: { analysis: 'vision-a' }, modelImageResolution: '1280' }),
    backend: ref({
      models: [
        { id: 'vision-a', imageIn: 'yes', toolCalls: 'yes' },
        { id: 'text-only', imageIn: 'no', toolCalls: 'yes' },
        { id: 'depth-s', imageIn: 'no', toolCalls: 'no', tasks: ['image-to-depth'] },
        { id: 'up-x', imageIn: 'no', toolCalls: 'no', tasks: ['image-upscale'] },
        { id: 'img-m', imageIn: 'no', toolCalls: 'no', tasks: ['text-to-image'] },
      ],
    }),
  }),
}))

beforeEach(() => {
  snapshot.value = { ...EMPTY_SNAPSHOT }
  capture.mockClear()
  cancel.mockClear()
  captureImage.mockClear()
  usePlaygroundHistory().runs.value = []
  usePlaygroundHistory().clearView()
})

describe('playground window', () => {
  it('lists every pipeline group in one full-height sidebar', () => {
    const wrapper = mount(PlaygroundWindow)
    const labels = wrapper.findAll('[data-testid="playground-nav"] button').map((b) => b.text())
    expect(labels.slice(0, 4)).toEqual(['Ask', 'Point', 'Pose', 'Depth'])
    expect(labels).toHaveLength(19)
    expect(wrapper.findAll('[data-testid="playground-nav"] h2').map((h) => h.text())).toEqual([
      'Images',
      'Audio',
      'Video',
    ])
    wrapper.unmount()
  })

  it('shows a placeholder for pipelines that are not ready', async () => {
    const wrapper = mount(PlaygroundWindow)
    await wrapper.get('[data-testid="pipeline-edit"]').trigger('click')
    expect(wrapper.find('[data-testid="playground-run"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="playground-params"]').text()).toContain('Coming later')
    wrapper.unmount()
  })

  it('offers only models that advertise vision and tools', () => {
    const wrapper = mount(PlaygroundWindow)
    expect(wrapper.text()).toContain('vision-a')
    expect(wrapper.text()).not.toContain('text-only')
    wrapper.unmount()
  })

  it('runs the chosen target with this window model and resolution', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('textarea').setValue('Where is Save?')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    expect(capture).toHaveBeenCalledWith(
      'Where is Save?',
      { kind: 'screen', id: 1 },
      { model: 'vision-a', resolution: '1280' },
    )
    wrapper.unmount()
  })

  it('records finished runs and restores one from the history strip', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    snapshot.value = {
      ...EMPTY_SNAPSHOT,
      status: 'done',
      id: 'insight-1-1',
      question: 'Earlier question',
      answer: 'Earlier answer',
      screenshot: 'data:image/png;base64,AA==',
    }
    await flushPromises()
    expect(wrapper.findAll('[data-testid="history-run"]')).toHaveLength(1)
    snapshot.value = { ...EMPTY_SNAPSHOT }
    await flushPromises()
    await wrapper.get('[data-testid="history-run"]').trigger('click')
    expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe('Earlier question')
    expect(wrapper.get('[data-testid="playground-canvas"]').text()).toContain('Earlier answer')
    expect(wrapper.find('[data-testid="result-cancel"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('cancels a running analysis with Escape', async () => {
    const wrapper = mount(PlaygroundWindow, { attachTo: document.body })
    snapshot.value = { ...EMPTY_SNAPSHOT, status: 'asking', id: 'insight-2-2' }
    await flushPromises()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(cancel).toHaveBeenCalled()
    wrapper.unmount()
  })

  it('shapes the question for Point', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-point"]').trigger('click')
    await wrapper.get('textarea').setValue('the save button')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    expect(capture.mock.calls[0]![0]).toContain('Locate: the save button')
    wrapper.unmount()
  })

  it('runs Depth through the depth model, not the chat model', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-depth"]').trigger('click')
    expect(wrapper.text()).toContain('depth-s')
    expect(wrapper.findAll('textarea')).toHaveLength(0)
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_depth', {
      input: { target: { kind: 'screen', id: 1 }, model: 'depth-s' },
    })
    expect(capture).not.toHaveBeenCalled()
    expect(wrapper.findAll('[data-testid="compare-output"]')).toHaveLength(1)
    expect(wrapper.get('[data-testid="depth-result"]').text()).toContain('not meters')
    wrapper.unmount()
  })

  it('creates an image from a prompt without a source', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-create"]').trigger('click')
    expect(wrapper.text()).toContain('img-m')
    expect(wrapper.find('[aria-label="Source"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="playground-run"]').attributes('disabled')).toBeDefined()
    await wrapper.get('textarea').setValue('a red fox')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_create', {
      input: { prompt: 'a red fox', size: '1024x1024', model: 'img-m' },
    })
    expect(wrapper.findAll('[data-testid="create-image"]')).toHaveLength(1)
    wrapper.unmount()
  })

  it('runs Upscale with the upscale model and chosen scale', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-upscale"]').trigger('click')
    expect(wrapper.text()).toContain('up-x')
    expect(wrapper.text()).not.toContain('depth-s')
    await wrapper.get('[aria-label="Scale"] [role="radio"]:nth-child(2)').trigger('click')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_upscale', {
      input: { target: { kind: 'screen', id: 1 }, model: 'up-x', scale: 4 },
    })
    expect(wrapper.get('[data-testid="upscale-result"]').text()).toContain('2×2 to 8×8')
    wrapper.unmount()
  })

  it('runs an imported image without a capture target', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[aria-label="Source"] [role="radio"]:nth-child(3)').trigger('click')
    expect(wrapper.get('[data-testid="playground-run"]').attributes('disabled')).toBeDefined()
    const input = wrapper.get('input[type="file"]')
    Object.defineProperty(input.element, 'files', {
      value: [new File(['x'], 'shot.png', { type: 'image/png' })],
    })
    await input.trigger('change')
    await flushPromises()
    expect(wrapper.find('[data-testid="file-preview"]').exists()).toBe(true)
    await wrapper.get('textarea').setValue('What is this?')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    expect(captureImage).toHaveBeenCalledWith('What is this?', 'data:image/png;base64,AA==', {
      model: 'vision-a',
      resolution: '1280',
    })
    expect(capture).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})