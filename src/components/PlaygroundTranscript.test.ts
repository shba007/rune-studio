import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vite-plus/test'
import PlaygroundTranscript from './PlaygroundTranscript.vue'

describe('transcript', () => {
  it('copies the transcript and reports success', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
    const wrapper = mount(PlaygroundTranscript, {
      props: {
        state: { status: 'done', error: '', result: { text: 'hello', model: 'm', elapsedMs: 500 } },
      },
    })
    await wrapper.get('button').trigger('click')
    expect(writeText).toHaveBeenCalledWith('hello')
    expect(wrapper.text()).toContain('Copied')
    wrapper.unmount()
  })
})