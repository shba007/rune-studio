import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'

import PlaygroundResult from './PlaygroundResult.vue'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { AnalysisSnapshot } from '@/types/protocol'

const done: AnalysisSnapshot = {
  ...EMPTY_SNAPSHOT,
  status: 'done',
  id: 'insight-1-1',
  answer: 'A save button.',
  screenshot: 'data:image/png;base64,AA==',
  findings: [
    {
      n: 1,
      label: 'Save',
      kind: 'control',
      group: 'entities',
      box: [100, 200, 300, 400],
      description: 'd',
      evidence: 'e',
      point: null,
    },
  ],
}

describe('playground result', () => {
  it('draws numbered boxes over the frame as percentages', () => {
    const wrapper = mount(PlaygroundResult, { props: { snapshot: done, live: true } })
    const box = wrapper.get('[data-marker="1"]')
    expect(box.attributes('style')).toContain('left: 10%')
    expect(box.attributes('style')).toContain('width: 20%')
    expect(wrapper.get('[data-testid="result-stage"]').text()).toBe('DONE')
    expect(wrapper.text()).toContain('A save button.')
  })

  it('offers cancel only while a live run is busy', async () => {
    const busy = { ...done, status: 'asking' as const }
    const wrapper = mount(PlaygroundResult, { props: { snapshot: busy, live: true } })
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
    await wrapper.setProps({ live: false })
    expect(wrapper.find('[data-testid="result-cancel"]').exists()).toBe(false)
  })

  it('shows a failure message', () => {
    const wrapper = mount(PlaygroundResult, {
      props: { snapshot: { ...EMPTY_SNAPSHOT, status: 'error', error: 'boom' }, live: true },
    })
    expect(wrapper.get('[role="alert"]').text()).toContain('boom')
  })
})