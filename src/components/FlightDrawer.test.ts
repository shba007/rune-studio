import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vite-plus/test'

import type { TabId } from '@/config/flight'

import FlightDrawer from './FlightDrawer.vue'
import { makeCall, makeHeader, makeStep } from './flightTestKit'

type Loader = (callId: number) => Promise<{ request: string; response: string } | null>

const typing = makeStep(2, 5, {
  tool: 'type_text',
  argsJson: '{"text":"hunter2"}',
  reason: 'Type hunter2',
})

function mountDrawer(over: Record<string, unknown> = {}) {
  return mount(FlightDrawer, {
    props: {
      open: true,
      tabs: ['step', 'api'] as TabId[],
      tab: 'step' as TabId,
      step: typing,
      index: 1,
      header: makeHeader(),
      typed: ['hunter2'],
      calls: [],
      events: [],
      show: false,
      developer: true,
      screenshot: null,
      loadBodies: vi.fn<Loader>(async () => null),
      ...over,
    },
  })
}

describe('FlightDrawer', () => {
  it('lists exactly the tabs it is given', () => {
    const wrapper = mountDrawer()
    expect(wrapper.findAll('[role="tab"]').map((tab) => tab.text())).toEqual(['Step', 'API'])
    expect(wrapper.find('[data-testid="tab-timing"]').exists()).toBe(false)
  })

  it('hides typed text until SHOW, and asks the player to toggle it', async () => {
    const wrapper = mountDrawer()
    expect(wrapper.text()).not.toContain('hunter2')
    expect(wrapper.find('[data-testid="args"]').text()).toContain('7 characters')
    await wrapper.find('[data-testid="show-typed"]').trigger('click')
    expect(wrapper.emitted('toggle-show')).toHaveLength(1)
    await wrapper.setProps({ show: true })
    expect(wrapper.text()).toContain('hunter2')
  })

  it('shows the stored JSON behind RAW, with the typed text still hidden', async () => {
    const wrapper = mountDrawer()
    await wrapper.find('[data-testid="raw-toggle"]').trigger('click')
    const raw = wrapper.find('[data-testid="raw"]').text()
    expect(raw).toContain('"tool": "type_text"')
    expect(raw).not.toContain('hunter2')
    expect(wrapper.find('[data-testid="copy"]').exists()).toBe(true)
  })

  it('is closed and out of the tab order when not open', () => {
    const wrapper = mountDrawer({ open: false })
    expect(wrapper.find('[data-testid="drawer"]').attributes('aria-hidden')).toBe('true')
  })

  it('draws only a few rows of a 200-call list, and loads a body only when a row is opened', async () => {
    const calls = Array.from({ length: 200 }, (_, index) => makeCall(index + 1, 2))
    const loadBodies = vi.fn<Loader>(async () => ({
      request: '{"typed":"hunter2"}',
      response: '{"ok":true}',
    }))
    const wrapper = mountDrawer({ tab: 'api', calls, loadBodies })
    const rows = wrapper.findAll('[data-testid="call"]')
    expect(rows.length).toBeGreaterThan(0)
    expect(rows.length).toBeLessThan(20)
    expect(loadBodies).not.toHaveBeenCalled()
    await rows[0]!.trigger('click')
    await vi.waitFor(() => expect(wrapper.find('[data-testid="bodies"] pre').exists()).toBe(true))
    expect(loadBodies).toHaveBeenCalledWith(1)
    expect(wrapper.find('[data-testid="bodies"]').text()).not.toContain('hunter2')
  })

  it('says when a body was cut', async () => {
    const wrapper = mountDrawer({
      tab: 'api',
      calls: [makeCall(1, 2, { truncated: true })],
      loadBodies: vi.fn<Loader>(async () => ({ request: '{}', response: '{}' })),
    })
    await wrapper.find('[data-testid="call"]').trigger('click')
    await vi.waitFor(() => expect(wrapper.find('[data-testid="truncated"]').exists()).toBe(true))
  })

  it('shows served model and swap warning when model differs from requested', async () => {
    const wrapper = mountDrawer({
      tab: 'api',
      calls: [
        makeCall(1, 2, {
          model: 'requested-model',
          servedModel: 'different-served-model',
          swapMs: 1250,
        }),
      ],
      loadBodies: vi.fn<Loader>(async () => ({ request: '{}', response: '{}' })),
    })
    await wrapper.find('[data-testid="call"]').trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('[data-testid="call-model-info"]').exists()).toBe(true),
    )
    expect(wrapper.find('[data-testid="model-mismatch-warning"]').text()).toContain(
      'Served by different-served-model (requested requested-model)',
    )
    expect(wrapper.find('[data-testid="call-model-info"]').text()).toContain('Swap: 1250 ms')
  })
})