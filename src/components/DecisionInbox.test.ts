import { shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it } from 'vite-plus/test'
import DecisionInbox from './DecisionInbox.vue'
import type { DecisionModel } from '@/composables/useDecisions'

it('allows compact decisions to grow naturally without a nested scroll area', () => {
  const model = {
    current: ref(undefined),
    entries: ref([]),
    error: ref(''),
  } as unknown as DecisionModel
  const wrapper = shallowMount(DecisionInbox, { props: { compact: true, model } })
  expect((wrapper.element as HTMLElement).style.maxHeight).toBe('')
  expect(wrapper.classes()).not.toContain('overflow-y-auto')
  wrapper.unmount()
})