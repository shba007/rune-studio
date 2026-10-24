import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'

import type { ActionLayer } from '@/config/flight'

import FlightStage from './FlightStage.vue'
import { stubResizeObserver } from './flightTestKit'

vi.mock('@videojs/html/video/player', () => ({}))
vi.mock('@videojs/html/ui/container', () => ({}))

beforeEach(() => stubResizeObserver(800, 450))

async function mountStage(over: Record<string, unknown> = {}) {
  const wrapper = mount(FlightStage, {
    props: {
      videoSrc: null,
      still: 'asset://shot.jpg',
      stillMissing: false,
      layer: { kind: 'none' } as ActionLayer,
      layerOn: true,
      cursor: null,
      badge: 'STEP 1 / 3 · CLICK',
      note: null,
      ...over,
    },
  })
  await flushPromises()
  return wrapper
}

describe('FlightStage', () => {
  it('shows the screenshot when there is no video', async () => {
    const wrapper = await mountStage()
    expect(wrapper.find('[data-testid="still"]').attributes('src')).toBe('asset://shot.jpg')
    expect(wrapper.find('video').exists()).toBe(false)
  })

  it('says so when the screenshot was deleted, instead of failing', async () => {
    const wrapper = await mountStage({ still: null, stillMissing: true })
    expect(wrapper.find('[data-testid="no-picture"]').text()).toContain('deleted')
  })

  it('draws the ripple at the click point and the step badge', async () => {
    const wrapper = await mountStage({ layer: { kind: 'click', x: 250, y: 500, label: 'CLICK' } })
    const cursor = wrapper.find('[data-testid="cursor"]')
    expect(cursor.attributes('style')).toContain('left: calc(25% - 1px)')
    expect(cursor.attributes('style')).toContain('top: calc(50% - 1px)')
    expect(cursor.get('img').attributes('src')).toBe('/cursor/default.svg')
    expect(cursor.find('[data-testid="ripple"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="badge"]').text()).toBe('STEP 1 / 3 · CLICK')
  })

  it('draws typed text as a caption, and nothing when the layer is off', async () => {
    const wrapper = await mountStage({ layer: { kind: 'type', text: '•••• (7 characters)' } })
    expect(wrapper.find('[data-testid="typed"]').text()).toBe('•••• (7 characters)')
    await wrapper.setProps({ layerOn: false })
    expect(wrapper.find('[data-testid="layer"]').exists()).toBe(false)
  })

  it('renders the Video.js player around the video element when there is one', async () => {
    const wrapper = await mountStage({ videoSrc: 'asset://session.mp4' })
    expect(wrapper.find('video-player video').attributes('src')).toBe('asset://session.mp4')
    expect(wrapper.find('[data-testid="still"]').exists()).toBe(false)
  })

  it('shows a note about the video', async () => {
    const wrapper = await mountStage({ note: 'This session has no video.' })
    expect(wrapper.find('[data-testid="note"]').text()).toContain('no video')
  })
})