import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vite-plus/test'

import { segments, stepStarts } from '@/config/flight'

import FlightTimeline from './FlightTimeline.vue'
import { START, makeStep, stubResizeObserver } from './flightTestKit'

beforeEach(() => stubResizeObserver(800, 80))

function props(count: number, extra: Record<string, unknown> = {}) {
  const steps = Array.from({ length: count }, (_, index) => makeStep(index + 1, index))
  const starts = stepStarts(START, steps)
  const total = count * 1000
  return {
    segments: segments(starts, total),
    steps,
    starts,
    total,
    timeMs: 0,
    events: [],
    runStartedUtc: START,
    retried: new Set<number>(),
    ...extra,
  }
}

describe('FlightTimeline', () => {
  it('draws one segment per step for a short run, and marks the current time', async () => {
    const wrapper = mount(FlightTimeline, { props: props(6, { timeMs: 2500 }) })
    await flushPromises()
    expect(wrapper.findAll('[data-testid="segment"]')).toHaveLength(6)
    expect(wrapper.findAll('[data-step-duration]').map((node) => node.text())).toEqual([
      '0:01',
      '0:01',
      '0:01',
      '0:01',
      '0:01',
      '0:00',
    ])
    expect(wrapper.find('[data-testid="track"]').attributes('aria-valuetext')).toBe(
      '0:02, step 3 of 6',
    )
  })

  it('draws only the segments in view when a long run is zoomed in', async () => {
    const wrapper = mount(FlightTimeline, { props: props(200) })
    await flushPromises()
    const track = wrapper.find('[data-testid="track"]')
    for (let notch = 0; notch < 12; notch++) await track.trigger('wheel', { deltaY: -100 })
    const drawn = wrapper.findAll('[data-testid="segment"]').length
    expect(drawn).toBeGreaterThan(0)
    expect(drawn).toBeLessThan(40)
  })

  it('seeks when the track is clicked', async () => {
    const wrapper = mount(FlightTimeline, { props: props(4) })
    await flushPromises()
    const track = wrapper.find('[data-testid="track"]')
    ;(track.element as HTMLElement).getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 800, height: 80, right: 800, bottom: 80, x: 0, y: 0 }) as DOMRect
    track.element.dispatchEvent(new MouseEvent('pointerdown', { clientX: 400, bubbles: true }))
    const seeks = wrapper.emitted('seek')
    expect(seeks).toBeTruthy()
    expect(seeks![0]![0]).toBeCloseTo(2000, -2)
  })

  it('marks a retried step, draws event markers and shows errors in red only', async () => {
    const wrapper = mount(FlightTimeline, {
      props: props(4, {
        retried: new Set([1]),
        events: [
          { id: 1, atUtc: '2026-10-20T10:00:01Z', kind: 'steer', dataJson: null },
          { id: 2, atUtc: '2026-10-20T10:00:02Z', kind: 'invalid_answer', dataJson: null },
        ],
      }),
    })
    await flushPromises()
    expect(wrapper.findAll('[data-testid="retry-tick"]')).toHaveLength(1)
    const marks = wrapper.findAll('[data-testid="marker"]')
    expect(marks.map((mark) => mark.text())).toEqual(['STEER', 'INVALID'])
    expect(marks[0]!.classes().join(' ')).not.toContain('text-error')
    expect(marks[1]!.classes().join(' ')).toContain('text-error')
  })
  it('ignores a segment without a corresponding step', async () => {
    const input = props(2)
    const wrapper = mount(FlightTimeline, { props: { ...input, steps: input.steps.slice(0, 1) } })
    await flushPromises()
    expect(wrapper.findAll('[data-testid="segment"]')).toHaveLength(1)
  })
})