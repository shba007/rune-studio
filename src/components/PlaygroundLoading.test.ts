import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'
import { nextTick } from 'vue'

import PlaygroundLoading from './PlaygroundLoading.vue'

afterEach(() => vi.useRealTimers())

describe('PlaygroundLoading', () => {
  it('updates elapsed time from the original start and emits cancellation', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-01-01T00:00:00Z'))
    const start = Date.now() - 32000
    const wrapper = mount(PlaygroundLoading, {
      props: { startedAt: start, label: 'Creating voice', cancellable: true },
    })
    expect(wrapper.get('[data-testid="loading-elapsed"]').text()).toBe('32.0 s')
    vi.advanceTimersByTime(500)
    await nextTick()
    expect(wrapper.get('[data-testid="loading-elapsed"]').text()).toBe('32.5 s')
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
    wrapper.unmount()
  })
})