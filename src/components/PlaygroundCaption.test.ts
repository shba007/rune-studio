import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vite-plus/test'
import PlaygroundCaption from './PlaygroundCaption.vue'

describe('PlaygroundCaption', () => {
  it('copies the caption and shows the selected image', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
    const wrapper = mount(PlaygroundCaption, {
      props: {
        state: {
          status: 'done',
          error: '',
          result: {
            image: 'data:image/jpeg;base64,AA==',
            caption: 'A red square.',
            model: 'vision',
            elapsedMs: 1000,
          },
        },
      },
    })
    expect(wrapper.get('img').attributes('src')).toContain('data:image/jpeg')
    await wrapper.get('button').trigger('click')
    expect(writeText).toHaveBeenCalledWith('A red square.')
    expect(wrapper.text()).toContain('Copied')
    wrapper.unmount()
  })
})