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
const empty = () =>
  ({ current: ref(undefined), entries: ref([]), error: ref('') }) as unknown as DecisionModel

it('tells an empty Goals page how to start, and stays silent when quiet', () => {
  const goals = shallowMount(DecisionInbox, { props: { goalsOnly: true, model: empty() } })
  expect(goals.text()).toContain('No goals yet. Use + New goal')
  goals.unmount()
  const quiet = shallowMount(DecisionInbox, { props: { quiet: true, model: empty() } })
  expect(quiet.text()).toBe('')
  quiet.unmount()
})