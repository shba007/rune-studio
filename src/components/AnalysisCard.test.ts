import { mount } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'

import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { useAnalysis } from '@/composables/useAnalysis'
import type { AnalysisSnapshot, Finding } from '@/types/protocol'

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
    close: vi.fn<(...args: unknown[]) => void>(),
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

describe('AnalysisCard states', () => {
  it('shows a dot skeleton and a Cancel button while it works, no spinner', () => {
    const { analysis, calls } = fake({ status: 'asking', question: 'what is here?' })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    expect(wrapper.findComponent(AnalysisAnswer).props('waiting')).toBe(true)
    expect(wrapper.findAll('.skeleton-dot')).toHaveLength(42)
    expect(wrapper.text()).toContain('what is here?')
    expect(wrapper.text()).toContain('READING')
    const cancel = wrapper.find('button[aria-label="Cancel analysis"]')
    expect(cancel.exists()).toBe(true)
    void cancel.trigger('click')
    expect(calls.cancel).toHaveBeenCalled()
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
    // The stack scrolls inside the pill, and labels truncate instead of widening a row.
    expect(wrapper.classes()).toEqual(expect.arrayContaining(['max-h-full', 'overflow-y-auto']))
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

  it('closing the analysis calls close', async () => {
    const { analysis, calls } = fake({ status: 'done', answer: 'x' })
    const wrapper = mount(AnalysisCard, { props: { analysis } })
    await wrapper.find('button[aria-label="Close the analysis"]').trigger('click')
    expect(calls.close).toHaveBeenCalled()
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
    expect(wrapper.text()).toContain('Screen analysis')
    expect(wrapper.find('[data-testid="stage"]').text()).toBe('READING')
    expect(wrapper.find('[data-testid="timer"]').text()).toBe('00:14')
    expect(wrapper.text()).toContain('Ctrl + Alt + Escape cancels')
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