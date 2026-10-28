import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent, h } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import { useModelActivity } from './useModelActivity'

const harness = vi.hoisted(() => ({
  handler: null as null | ((event: { payload: unknown }) => void),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (_name: string, handler: typeof harness.handler) => {
    harness.handler = handler
    return () => {
      harness.handler = null
    }
  },
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: async () => [] }))

it('shows distinct concurrent models and removes them when their calls end', async () => {
  const component = defineComponent({
    setup() {
      const activity = useModelActivity()
      return () => h('div', activity.models.value.join(','))
    },
  })
  const wrapper = mount(component)
  await flushPromises()
  harness.handler!({
    payload: [
      { id: 1, model: 'qwen-27b', reported: false },
      { id: 2, model: 'gemma-4b', reported: true },
      { id: 3, model: 'qwen-27b', reported: false },
    ],
  })
  await flushPromises()
  expect(wrapper.text()).toBe('qwen-27b,gemma-4b')
  harness.handler!({ payload: [{ id: 2, model: 'gemma-4b', reported: true }] })
  await flushPromises()
  expect(wrapper.text()).toBe('gemma-4b')
  harness.handler!({ payload: [] })
  await flushPromises()
  expect(wrapper.text()).toBe('')
  wrapper.unmount()
})