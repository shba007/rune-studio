import { mount, flushPromises } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'

import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { useAnalysis } from '@/composables/useAnalysis'
import type { AnalysisSnapshot, Finding } from '@/types/protocol'

import { invoke } from '@tauri-apps/api/core'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn<typeof invoke>().mockResolvedValue([]) }))
import AnalysisAnswer from './AnalysisAnswer.vue'
import AnalysisBar from './AnalysisBar.vue'
import AnalysisCard from './AnalysisCard.vue'
import AnalysisFindingRow from './AnalysisFindingRow.vue'
import OverlayBarShell from './OverlayBarShell.vue'

const finding = (n: number, over: Partial<Finding> = {}): Finding => ({
  n,
  label: `Thing ${n}`,
  kind: 'control',
  box: [10 * n, 10, 10 * n + 50, 60],
  description: `Description ${n}`,
  evidence: `Evidence ${n}`,
  point: null,
  ...over,
})

function fake(over: Partial<AnalysisSnapshot> = {}) {
  const snapshot = ref<AnalysisSnapshot>({ ...EMPTY_SNAPSHOT, ...over })
  const calls = {
    capture: vi.fn<(...args: unknown[]) => void>(),
    ask: vi.fn<(...args: unknown[]) => void>(),
    cancel: vi.fn<(...args: unknown[]) => void>(),
    close: vi.fn<(...args: unknown[]) => Promise<boolean>>(async () => true),
    select: vi.fn<(...args: unknown[]) => void>(),
  }
  const analysis = {
    snapshot,
    notice: ref<string | null>(null),
    busy: computed(() => ['capturing', 'asking'].includes(snapshot.value.status)),
    ...calls,
  } as unknown as ReturnType<typeof useAnalysis>
  return { analysis, snapshot, calls }
}

const press = (key: string) => window.dispatchEvent(new KeyboardEvent('keydown', { key }))
const submitFromComposer = (wrapper: { vm: unknown }, question: string) =>
  (wrapper.vm as { submit: (question: string) => Promise<boolean> }).submit(question)

describe('AnalysisCard states', () => {
  it('warns about rejected pose data without hiding valid landmarks', () => {
    const { analysis } = fake({
      status: 'done',
      findings: [
        finding(1, {
          group: 'structure',
          box: null,
          pose: { keypoints: [{ label: 'wrist', x: 300, y: 400 }], connections: [], rejected: 1 },
        }),
      ],
    })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.text()).toContain('Some pose data was invalid and omitted')
    expect(wrapper.find('[aria-label="Finding 1: Thing 1"]').exists()).toBe(true)
    wrapper.unmount()
  })
  it('preselects the primary screen without capturing and preserves a manually selected window on refresh', async () => {
    const options = [
      { target: { kind: 'screen', id: 1 }, label: 'Secondary', primary: false },
      { target: { kind: 'screen', id: 2 }, label: 'Primary', primary: true },
      { target: { kind: 'window', id: 42 }, label: 'Acceptance', primary: false },
    ]
    vi.mocked(invoke).mockResolvedValueOnce(options).mockResolvedValueOnce(options)
    const { analysis, calls } = fake()
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    await flushPromises()
    expect(calls.capture).not.toHaveBeenCalled()
    await submitFromComposer(wrapper, 'Describe this screen')
    expect(calls.capture).toHaveBeenLastCalledWith('Describe this screen', {
      kind: 'screen',
      id: 2,
    })
    await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
    await wrapper
      .findAll('button')
      .find((button) => button.text().includes('Acceptance'))!
      .trigger('click')
    await wrapper.get('[aria-label="Refresh capture targets"]').trigger('click')
    await flushPromises()
    await submitFromComposer(wrapper, 'Describe this window')
    expect(calls.capture).toHaveBeenLastCalledWith('Describe this window', {
      kind: 'window',
      id: 42,
    })
    wrapper.unmount()
  })
  it('clears a disappeared target and refuses submission from the main composer', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([{ target: { kind: 'window', id: 42 }, label: 'Window — Acceptance' }])
      .mockResolvedValueOnce([])
    const { analysis, calls } = fake()
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    await flushPromises()
    await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
    await wrapper
      .findAll('button')
      .find((button) => button.text().includes('Acceptance'))!
      .trigger('click')
    await wrapper.get('[aria-label="Refresh capture targets"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('input').exists()).toBe(false)
    expect(await submitFromComposer(wrapper, 'What changed?')).toBe(false)
    expect(calls.capture).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('captures only the explicitly selected target and presents the same returned frame', async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      { target: { kind: 'window', id: 42 }, label: 'Window — Acceptance' },
    ])
    const { analysis, calls, snapshot } = fake()
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    await flushPromises()
    expect(calls.capture).not.toHaveBeenCalled()
    await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
    await wrapper
      .findAll('button')
      .find((button) => button.text().includes('Acceptance'))!
      .trigger('click')
    expect(wrapper.find('input').exists()).toBe(false)
    calls.capture.mockImplementationOnce(() => new Promise<void>(() => {}))
    expect(await submitFromComposer(wrapper, 'What failed?')).toBe(true)
    expect(calls.capture).toHaveBeenCalledWith('What failed?', { kind: 'window', id: 42 })
    snapshot.value = {
      ...snapshot.value,
      status: 'done',
      id: 'insight-test',
      screenshot: 'data:image/jpeg;base64,aGVsbG8=',
      model: 'explicit-vision',
      createdUtc: 'capture-time',
    }
    await nextTick()
    expect(wrapper.get('img').attributes('src')).toBe('data:image/jpeg;base64,aGVsbG8=')
    expect(wrapper.text()).toContain('capture-time')
    expect(wrapper.text()).toContain('not verified facts')
    wrapper.unmount()
  })

  it('keeps the question while loading without duplicating the orb or composer stop control', () => {
    const { analysis } = fake({ status: 'asking', question: 'what is here?' })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.findComponent(AnalysisAnswer).exists()).toBe(false)
    expect(wrapper.text()).not.toContain('Reading your capture')
    expect(wrapper.text()).toContain('what is here?')
    expect(wrapper.find('p.whitespace-pre-wrap').classes()).not.toContain('truncate')
    expect(wrapper.text()).toContain('READING')
    const cancel = wrapper.find('button[aria-label="Cancel analysis"]')
    expect(cancel.exists()).toBe(false)
    expect(wrapper.find('input').exists()).toBe(false)
  })

  it('shows the answer clamped to four lines with MORE and LESS', async () => {
    const { analysis } = fake({ status: 'done', answer: 'word '.repeat(60) })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    const text = wrapper.find('section[aria-label="Answer"] p')
    expect(text.classes()).toContain('line-clamp-4')
    const toggle = wrapper.find('section[aria-label="Answer"] button')
    expect(toggle.text()).toBe('More')
    await toggle.trigger('click')
    expect(text.classes()).not.toContain('line-clamp-4')
    expect(toggle.text()).toBe('Less')
  })

  it('has no MORE for a short answer', () => {
    const { analysis } = fake({ status: 'done', answer: 'A settings window.' })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.find('section[aria-label="Answer"] button').exists()).toBe(false)
  })

  it('lists the findings, one row each, and shows eight without growing past its scroll container', () => {
    const findings = Array.from({ length: 8 }, (_, i) => finding(i + 1, { label: 'L'.repeat(24) }))
    const { analysis } = fake({ status: 'done', answer: 'x', findings })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.findAllComponents(AnalysisFindingRow)).toHaveLength(8)
    expect(wrapper.classes()).toEqual(expect.arrayContaining(['max-h-[480px]', 'overflow-y-auto']))
    expect(wrapper.find('li span.truncate').exists()).toBe(true)
  })

  it('shows an error card with Retry that asks the failed question again', async () => {
    const { analysis, calls } = fake({
      status: 'error',
      error: 'the model did not answer',
      question: 'what changed?',
      monitor: { id: 1, x: 0, y: 0, width: 10, height: 10, scaleFactor: 1, primary: true },
    })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    const alert = wrapper.find('[role="alert"]')
    expect(alert.text()).toContain('the model did not answer')
    await alert.find('button').trigger('click')
    expect(calls.ask).toHaveBeenCalledWith('what changed?')
  })

  it('says SCREEN CHANGED in the header once the boxes are gone, and keeps the findings', () => {
    const { analysis } = fake({ status: 'done', stale: true, answer: 'x', findings: [finding(1)] })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.text()).toContain('Changed')
    expect(wrapper.findAllComponents(AnalysisFindingRow)).toHaveLength(1)
  })

  it.each(['idle', 'asking', 'done'] as const)(
    'closing a %s analysis explicitly dismisses the panel',
    async (status) => {
      const { analysis, calls } = fake({ status, answer: 'x' })
      const wrapper = mount(AnalysisCard, { props: { analysis } })
      await wrapper.find('button[aria-label="Close the analysis"]').trigger('click')
      expect(calls.close).toHaveBeenCalled()
      await flushPromises()
      expect(wrapper.emitted('close')).toHaveLength(1)
      wrapper.unmount()
    },
  )
  it('keeps the panel open if native close fails', async () => {
    const { analysis, calls } = fake()
    calls.close.mockResolvedValueOnce(false)
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    await wrapper.get('[aria-label="Close the analysis"]').trigger('click')
    await flushPromises()
    expect(wrapper.emitted('close')).toBeUndefined()
    wrapper.unmount()
  })
})

describe('the findings accordion', () => {
  it('opens one at a time, picks its marker and closes on a second tap', async () => {
    const { analysis, calls } = fake({
      status: 'done',
      answer: 'x',
      findings: [finding(1), finding(2)],
    })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    const rows = () => wrapper.findAllComponents(AnalysisFindingRow)
    await rows()[0]!.find('button').trigger('click')
    expect(rows()[0]!.text()).toContain('Evidence 1')
    expect(calls.select).toHaveBeenLastCalledWith(1)
    await rows()[1]!.find('button').trigger('click')
    expect(rows()[0]!.text()).not.toContain('Evidence 1')
    expect(rows()[1]!.text()).toContain('Evidence 2')
    expect(calls.select).toHaveBeenLastCalledWith(2)
    await rows()[1]!.find('button').trigger('click')
    expect(rows()[1]!.text()).not.toContain('Evidence 2')
    expect(calls.select).toHaveBeenLastCalledWith(null)
  })

  it('shows evidence as plain text, never as HTML', async () => {
    const { analysis } = fake({
      status: 'done',
      answer: 'x',
      findings: [finding(1, { evidence: '<img src=x onerror=alert(1)>' })],
    })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    await wrapper.find('li button').trigger('click')
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.text()).toContain('<img src=x onerror=alert(1)>')
  })

  it('marks a finding without a box with a dashed badge and says so', () => {
    const { analysis } = fake({
      status: 'done',
      answer: 'x',
      findings: [finding(1, { box: null })],
    })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.find('li span.border-dashed').exists()).toBe(true)
    expect(wrapper.find('li button').attributes('aria-label')).toContain('no position')
  })

  it('escapes in order: the open row, the selection, then the cards', async () => {
    const { analysis, snapshot, calls } = fake({
      status: 'done',
      answer: 'x',
      findings: [finding(1)],
    })
    const wrapper = mount(AnalysisCard, { props: { analysis }, attachTo: document.body })
    await wrapper.find('li button').trigger('click')
    snapshot.value = { ...snapshot.value, selected: 1 }
    await nextTick()
    press('Escape')
    await nextTick()
    expect(wrapper.text()).not.toContain('Evidence 1')
    expect(calls.close).not.toHaveBeenCalled()
    press('Escape')
    await nextTick()
    expect(calls.select).toHaveBeenLastCalledWith(null)
    expect(calls.close).not.toHaveBeenCalled()
    snapshot.value = { ...snapshot.value, selected: null }
    await nextTick()
    press('Escape')
    await nextTick()
    expect(calls.close).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })
})

describe('reduced motion', () => {
  it('renders static dots; the wave exists only inside a no-preference media block', () => {
    const wrapper = mount(AnalysisAnswer, { props: { text: '', waiting: true } })
    expect(wrapper.find('.skeleton-dot').exists()).toBe(true)
  })
})

describe('the status bar', () => {
  it('renders the Rune bar without any agent-specific part', () => {
    const wrapper = mount(AnalysisBar, {
      props: { status: 'asking', elapsedMs: 14_300, hotkey: 'Ctrl + Alt + Escape' },
    })
    expect(wrapper.text()).toContain('Visual analysis')
    expect(wrapper.find('[data-testid="stage"]').text()).toBe('READING')
    expect(wrapper.find('[data-testid="timer"]').text()).toBe('00:14')
    expect(wrapper.text()).toContain('Ctrl + Alt + Escape')
    expect(wrapper.text()).not.toContain('cancels')
    expect(wrapper.text()).not.toContain('in control')
    expect(wrapper.find('button').exists()).toBe(false)
  })

  it('uses the same shell as the control bar, so control mode looks the same', () => {
    const shell = mount(OverlayBarShell)
    expect(shell.classes()).toEqual(
      expect.arrayContaining(['absolute', 'top-6', 'h-10', 'rounded-full', 'bg-black/85']),
    )
    const bar = mount(AnalysisBar, { props: { status: 'capturing', elapsedMs: 0, hotkey: '' } })
    expect(bar.classes()).toEqual(expect.arrayContaining(['top-6', 'h-10', 'bg-black/85']))
  })

  it('names every end state', () => {
    for (const [status, word] of [
      ['done', 'DONE'],
      ['cancelled', 'CANCELLED'],
      ['error', 'FAILED'],
      ['capturing', 'LOOKING'],
    ] as const) {
      const wrapper = mount(AnalysisBar, { props: { status, elapsedMs: 0, hotkey: '' } })
      expect(wrapper.find('[data-testid="stage"]').text()).toBe(word)
    }
  })
})