import { shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import DecisionInbox from './DecisionInbox.vue'
import type { DecisionModel } from '@/composables/useDecisions'

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

it('keeps the compact scroll area independent of floating-window resize and adapts to the screen', async () => {
  const screenHeight = vi.spyOn(window.screen, 'availHeight', 'get').mockReturnValue(900)
  const model = {
    current: ref(undefined),
    entries: ref([]),
    error: ref(''),
  } as unknown as DecisionModel
  const wrapper = shallowMount(DecisionInbox, { props: { compact: true, model } })
  expect((wrapper.element as HTMLElement).style.maxHeight).toBe('520px')
  vi.stubGlobal('innerHeight', 128)
  window.dispatchEvent(new Event('resize'))
  await wrapper.vm.$nextTick()
  expect((wrapper.element as HTMLElement).style.maxHeight).toBe('520px')
  expect(wrapper.classes()).toContain('overflow-y-auto')
  screenHeight.mockReturnValue(600)
  window.dispatchEvent(new Event('resize'))
  await wrapper.vm.$nextTick()
  expect((wrapper.element as HTMLElement).style.maxHeight).toBe('360px')
  await wrapper.setProps({ compact: false })
  expect((wrapper.element as HTMLElement).style.maxHeight).toBe('')
  wrapper.unmount()
})