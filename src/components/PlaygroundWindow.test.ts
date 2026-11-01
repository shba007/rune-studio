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
let pendingSpeech: Promise<unknown> | null = null
let pendingCaption: Promise<unknown> | null = null
let pendingStart: Promise<number> | null = null
let translationReply: Promise<unknown> | null = null
let translationError = ''
let noVerifiedAudio = false
vi.mock('@tauri-apps/api/core', () => ({
  invoke: async (command: string, args?: unknown) => {
    void invoked(command, args)
    if (command === 'playground_start_run') return pendingStart ?? 42
    if (command === 'playground_speech' && pendingSpeech) return pendingSpeech
    if (command === 'playground_translate_speech') {
      if (translationError) throw new Error(translationError)
      return translationReply ?? { text: 'Translated words', model: 'asr-a', elapsedMs: 700 }
    }
    if (command === 'playground_transcribe')
      return { text: 'Hello from audio', model: 'asr-a', elapsedMs: 1000 }
    if (command === 'playground_caption' && pendingCaption) return pendingCaption
    if (command === 'playground_caption')
      return {
        image: 'data:image/jpeg;base64,AA==',
        caption: 'A red square.',
        model: 'vision-a',
        elapsedMs: 500,
      }
    if (command === 'playground_speech')
      return {
        audio: 'data:audio/wav;base64,UklGRjAwMDBXQVZF',
        text: 'Hello there',
        model: 'voice-a',
        elapsedMs: 1200,
      }
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
      models: noVerifiedAudio
        ? [{ id: 'unverified-asr', imageIn: 'no', toolCalls: 'no', tasks: ['audio-to-text'] }]
        : [
            { id: 'vision-a', imageIn: 'yes', toolCalls: 'yes', tasks: ['text-image-to-text'] },
            { id: 'vision-no-tools', imageIn: 'yes', toolCalls: 'no', tasks: ['image-to-text'] },
            { id: 'text-only', imageIn: 'no', toolCalls: 'yes' },
            { id: 'depth-s', imageIn: 'no', toolCalls: 'no', tasks: ['image-to-depth'] },
            { id: 'up-x', imageIn: 'no', toolCalls: 'no', tasks: ['image-upscale'] },
            { id: 'img-m', imageIn: 'no', toolCalls: 'no', tasks: ['text-to-image'] },
            {
              id: 'voice-a',
              imageIn: 'no',
              toolCalls: 'no',
              tasks: ['text-to-audio'],
              audioOperations: { speechGeneration: { status: 'supported' } },
            },
            {
              id: 'asr-a',
              imageIn: 'no',
              toolCalls: 'no',
              tasks: ['audio-to-text'],
              audioOperations: {
                transcription: { status: 'supported' },
                speechTranslation: { status: 'supported' },
              },
            },
          ],
    }),
  }),
}))

beforeEach(() => {
  pendingSpeech = null
  pendingCaption = null
  pendingStart = null
  translationReply = null
  translationError = ''
  noVerifiedAudio = false
  invoked.mockClear()
  snapshot.value = { ...EMPTY_SNAPSHOT }
  capture.mockClear()
  cancel.mockClear()
  captureImage.mockClear()
  usePlaygroundHistory().clear()
})

describe('playground window', () => {
  it('restores a voice result and its input after switching pipelines', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-tts"]').trigger('click')
    await wrapper.get('#speech-text').setValue('Hello there')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-caption"]').trigger('click')
    await wrapper.get('[data-testid="history-run"]').trigger('click')
    expect(wrapper.get('[data-testid="speech-player"]').attributes('src')).toContain(
      'data:audio/wav',
    )
    expect(wrapper.get('[data-testid="history-details"]').text()).toContain('Text: Hello there')
    wrapper.unmount()
  })
  it('reopens a prior caption without replacing the live result', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-caption"]').trigger('click')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    pendingCaption = Promise.resolve({
      image: 'data:image/jpeg;base64,BB==',
      caption: 'A blue circle.',
      model: 'vision-a',
      elapsedMs: 700,
    })
    await wrapper.get('[aria-label="Detail"] [role="radio"]:nth-child(2)').trigger('click')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(wrapper.findAll('[data-testid="history-run"]')).toHaveLength(2)
    await wrapper.findAll('[data-testid="history-run"]')[1]!.trigger('click')
    expect(wrapper.get('[data-testid="caption-result"]').text()).toContain('A red square.')
    expect(wrapper.get('[data-testid="history-details"]').text()).toContain('Detail: concise')
    await wrapper.get('[data-testid="history-details"] button').trigger('click')
    expect(wrapper.get('[data-testid="caption-result"]').text()).toContain('A blue circle.')
    wrapper.unmount()
    expect(usePlaygroundHistory().runs.value).toHaveLength(0)
  })

  it('records Insight under the pipeline that started it after switching tabs', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-point"]').trigger('click')
    await wrapper.get('textarea').setValue('Save')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await wrapper.get('[data-testid="pipeline-caption"]').trigger('click')
    snapshot.value = {
      ...EMPTY_SNAPSHOT,
      status: 'done',
      id: 'point-result',
      question: 'Locate: Save. Give a box and a point at the centre for each match.',
      answer: 'Found',
    }
    await flushPromises()
    expect(usePlaygroundHistory().runs.value[0]?.pipeline).toBe('point')
    await wrapper.get('[data-testid="history-run"]').trigger('click')
    expect(wrapper.get('[data-testid="history-details"]').text()).toContain('Point')
    expect(wrapper.get('[data-testid="playground-canvas"]').text()).toContain('Found')
    wrapper.unmount()
  })
  it('cancels captioning and discards late results', async () => {
    let finish!: (value: unknown) => void
    pendingCaption = new Promise((resolve) => {
      finish = resolve
    })
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-caption"]').trigger('click')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    finish({ image: '', caption: 'late result', model: 'vision-a', elapsedMs: 5 })
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_cancel_run', { runId: 42 })
    expect(wrapper.text()).not.toContain('late result')
    expect(usePlaygroundHistory().runs.value).toHaveLength(0)
    wrapper.unmount()
  })

  it('sends an imported image for captioning', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-caption"]').trigger('click')
    await wrapper.get('[aria-label="Source"] [role="radio"]:nth-child(3)').trigger('click')
    const input = wrapper.get('input[aria-label="Image file"]')
    Object.defineProperty(input.element, 'files', {
      configurable: true,
      value: [new File(['image'], 'image.png', { type: 'image/png' })],
    })
    await input.trigger('change')
    await flushPromises()
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_caption', {
      runId: 42,
      input: { image: 'data:image/png;base64,AA==', model: 'vision-a', detail: 'concise' },
    })
    wrapper.unmount()
  })
  it('captions a selected screen with a vision model and chosen detail', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-caption"]').trigger('click')
    expect(wrapper.findAll('textarea')).toHaveLength(0)
    await wrapper.get('[aria-label="Detail"] [role="radio"]:nth-child(2)').trigger('click')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_caption', {
      runId: 42,
      input: { target: { kind: 'screen', id: 1 }, model: 'vision-a', detail: 'detailed' },
    })
    expect(wrapper.get('[data-testid="caption-result"]').text()).toContain('A red square.')
    wrapper.unmount()
  })
  it('does not offer unverified translation models', async () => {
    noVerifiedAudio = true
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-translate-speech"]').trigger('click')
    expect(wrapper.text()).toContain('No verified translation model')
    expect(wrapper.text()).toContain('Infer has not reported a model verified for this operation')
    expect(wrapper.text()).not.toContain('unverified-asr')
    expect(wrapper.get('[data-testid="playground-run"]').attributes('disabled')).toBeDefined()
    wrapper.unmount()
  })
  it('shows Infer errors when the model does not support translation', async () => {
    translationError =
      'Infer refused the translation request (400 Bad Request): Engine does not implement translation'
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-translate-speech"]').trigger('click')
    const input = wrapper.get('input[aria-label="WAV file"]')
    Object.defineProperty(input.element, 'files', {
      configurable: true,
      value: [new File(['audio'], 'sample.wav', { type: 'audio/wav' })],
    })
    await input.trigger('change')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.get('[role="alert"]').text()).toContain(
        'Engine does not implement translation',
      ),
    )
    expect(usePlaygroundHistory().runs.value).toHaveLength(0)
    wrapper.unmount()
  })
  it('translates WAV audio with a separate result and supports cancellation', async () => {
    let finish!: (value: unknown) => void
    translationReply = new Promise((resolve) => {
      finish = resolve
    })
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-translate-speech"]').trigger('click')
    expect(wrapper.text()).toContain(
      'Only models verified by Infer for speech translation are listed',
    )
    const input = wrapper.get('input[aria-label="WAV file"]')
    Object.defineProperty(input.element, 'files', {
      configurable: true,
      value: [new File(['audio'], 'sample.wav', { type: 'audio/wav' })],
    })
    await input.trigger('change')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await vi.waitFor(() =>
      expect(invoked).toHaveBeenCalledWith('playground_translate_speech', {
        runId: 42,
        input: { audio: expect.stringContaining('data:audio/wav;base64,'), model: 'asr-a' },
      }),
    )
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    expect(invoked).toHaveBeenCalledWith('playground_cancel_run', { runId: 42 })
    translationReply = null
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.get('[data-testid="transcript-result"]').text()).toContain('Translated words'),
    )
    finish({ text: 'Old translation', model: 'asr-a', elapsedMs: 3 })
    await flushPromises()
    expect(wrapper.text()).not.toContain('Old translation')
    await wrapper.get('[data-testid="pipeline-transcribe"]').trigger('click')
    expect(wrapper.find('[data-testid="transcript-result"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('cancels a run even while registration is pending', async () => {
    let finish!: (id: number) => void
    pendingStart = new Promise((resolve) => {
      finish = resolve
    })
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-tts"]').trigger('click')
    await wrapper.get('#speech-text').setValue('Hello there')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    finish(45)
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_cancel_run', { runId: 45 })
    expect(invoked).not.toHaveBeenCalledWith('playground_speech', expect.anything())
    wrapper.unmount()
  })
  it('cancels a model run and ignores its late result after a retry', async () => {
    let finish!: (result: unknown) => void
    pendingSpeech = new Promise((resolve) => {
      finish = resolve
    })
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-tts"]').trigger('click')
    await wrapper.get('#speech-text').setValue('Hello there')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    expect(invoked).toHaveBeenCalledWith('playground_cancel_run', { runId: 42 })
    pendingSpeech = null
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    finish({ audio: 'old', text: 'stale', model: 'old', elapsedMs: 1 })
    await flushPromises()
    expect(wrapper.get('[data-testid="speech-result"]').text()).toContain('Hello there')
    expect(wrapper.get('[data-testid="speech-result"]').text()).not.toContain('stale')
    wrapper.unmount()
  })
  it('uploads an audio file and displays the transcript', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-transcribe"]').trigger('click')
    expect(wrapper.text()).toContain('asr-a')
    expect(wrapper.get('[data-testid="playground-run"]').attributes('disabled')).toBeDefined()
    const input = wrapper.get('input[aria-label="WAV file"]')
    Object.defineProperty(input.element, 'files', {
      configurable: true,
      value: [new File(['audio'], 'sample.wav', { type: 'audio/wav' })],
    })
    await input.trigger('change')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await vi.waitFor(() =>
      expect(invoked).toHaveBeenCalledWith('playground_transcribe', {
        runId: 42,
        input: { audio: expect.stringContaining('data:audio/wav;base64,'), model: 'asr-a' },
      }),
    )
    await flushPromises()
    expect(wrapper.get('[data-testid="transcript-result"]').text()).toContain('Hello from audio')
    wrapper.unmount()
  })
  it('lists every pipeline group in one full-height sidebar', () => {
    const wrapper = mount(PlaygroundWindow)
    const labels = wrapper.findAll('[data-testid="playground-nav"] button').map((b) => b.text())
    expect(labels.slice(0, 5)).toEqual(['Ask', 'Point', 'Pose', 'Caption', 'Depth'])
    expect(labels).toHaveLength(21)
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
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
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
      runId: 42,
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
      runId: 42,
      input: { prompt: 'a red fox', size: '720x720', model: 'img-m' },
    })
    expect(wrapper.findAll('[data-testid="create-image"]')).toHaveLength(1)
    wrapper.unmount()
  })

  it('creates playable, downloadable speech with a speech model', async () => {
    const wrapper = mount(PlaygroundWindow)
    await flushPromises()
    await wrapper.get('[data-testid="pipeline-tts"]').trigger('click')
    expect(wrapper.text()).toContain('Create Voice')
    expect(wrapper.text()).toContain('voice-a')
    expect(wrapper.find('[aria-label="Source"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="playground-run"]').attributes('disabled')).toBeDefined()
    await wrapper.get('#speech-text').setValue('')
    const hint = wrapper
      .findAll('button')
      .find((button) => button.text() === 'Hello! Welcome to Rune Studio.')
    expect(hint).toBeDefined()
    await hint!.trigger('click')
    expect((wrapper.get('#speech-text').element as HTMLTextAreaElement).value).toBe(
      'Hello! Welcome to Rune Studio.',
    )
    await wrapper.get('#speech-text').setValue('é'.repeat(2049))
    expect(wrapper.get('[data-testid="playground-run"]').attributes('disabled')).toBeDefined()
    await wrapper.get('#speech-text').setValue('Hello there')
    await wrapper.get('[data-testid="playground-run"]').trigger('click')
    await flushPromises()
    expect(invoked).toHaveBeenCalledWith('playground_speech', {
      runId: 42,
      input: { text: 'Hello there', model: 'voice-a' },
    })
    expect(wrapper.get('[data-testid="speech-player"]').attributes('src')).toContain(
      'data:audio/wav',
    )
    expect(wrapper.get('[data-testid="speech-download"]').attributes('download')).toBe(
      'rune-voice.wav',
    )
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
      runId: 42,
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