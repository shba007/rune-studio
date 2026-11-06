import { mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import DownloadProgress from './DownloadProgress.vue'

it('reports per-file progress and clears rate when the stage changes', async () => {
  vi.useFakeTimers()
  const wrapper = mount(DownloadProgress, {
    props: { phase: 'downloading', filename: 'model.gguf', downloaded: 0, total: 100000000 },
  })
  vi.advanceTimersByTime(1000)
  await wrapper.setProps({ downloaded: 20000000 })
  expect(wrapper.text()).toContain('20%')
  expect(wrapper.text()).toContain('20.0 MB / 100.0 MB')
  expect(wrapper.text()).toContain('20.0 MB/s')
  expect(wrapper.text()).toContain('4s remaining')
  await wrapper.setProps({ phase: 'engine', filename: undefined, downloaded: 0, total: undefined })
  expect(wrapper.text()).toContain('Preparing engine')
  expect(wrapper.text()).not.toContain('/s')
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
  wrapper.unmount()
  vi.useRealTimers()
})