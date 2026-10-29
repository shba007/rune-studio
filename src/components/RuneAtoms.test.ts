import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'
import RuneButton from './RuneButton.vue'
import RuneChip from './RuneChip.vue'
import RuneDot from './RuneDot.vue'
import RuneIconButton from './RuneIconButton.vue'
import RuneNotice from './RuneNotice.vue'
import WorkEmpty from './WorkEmpty.vue'
import WorkRow from './WorkRow.vue'

describe('Rune atoms', () => {
  it('renders a pill button that defaults to a plain button', () => {
    const wrapper = mount(RuneButton, { slots: { default: 'Update' } })
    expect(wrapper.attributes('type')).toBe('button')
    expect(wrapper.classes()).toContain('rounded-full')
    expect(wrapper.classes()).toContain('h-10')
    expect(wrapper.text()).toBe('Update')
  })

  it('maps button variants and sizes to tone classes', () => {
    const danger = mount(RuneButton, { props: { variant: 'danger', size: 'sm' } })
    expect(danger.classes()).toContain('bg-attention')
    expect(danger.classes()).toContain('h-8')
    expect(mount(RuneButton, { props: { variant: 'primary' } }).classes()).toContain('bg-rune-ink')
  })

  it('keeps native disabled behaviour', async () => {
    const wrapper = mount(RuneButton, { attrs: { disabled: true } })
    expect(wrapper.attributes('disabled')).toBeDefined()
    await wrapper.trigger('click')
    expect(wrapper.emitted('click')).toBeUndefined()
  })

  it('names the icon button for assistive technology', () => {
    const wrapper = mount(RuneIconButton, { props: { label: 'Stop', variant: 'danger' } })
    expect(wrapper.attributes('aria-label')).toBe('Stop')
    expect(wrapper.classes()).toContain('rounded-full')
    expect(wrapper.classes()).toContain('bg-attention')
    expect(wrapper.classes()).toContain('size-10')
  })

  it('renders chips and dots by variant', () => {
    expect(mount(RuneChip, { props: { variant: 'red' } }).classes()).toContain('bg-attention')
    expect(mount(RuneDot, { props: { state: 'down' } }).classes()).toContain('bg-attention')
    expect(mount(RuneDot, { props: { state: 'busy' } }).classes()).toContain(
      'motion-safe:animate-pulse',
    )
  })

  it('opens an interactive work row and keeps a plain one inert', async () => {
    const live = mount(WorkRow, {
      props: { title: 'Plan trip', meta: 'Today', state: 'ok', interactive: true },
      slots: { actions: '<span>act</span>' },
    })
    expect(live.text()).toContain('Plan trip')
    expect(live.text()).toContain('act')
    await live.get('button').trigger('click')
    expect(live.emitted('open')).toHaveLength(1)
    expect(
      mount(WorkRow, { props: { title: 'Static' } })
        .find('button')
        .exists(),
    ).toBe(false)
  })

  it('announces notices by tone and renders the empty state', () => {
    expect(mount(RuneNotice, { props: { tone: 'error' } }).attributes('role')).toBe('alert')
    expect(mount(RuneNotice).attributes('role')).toBe('status')
    const empty = mount(WorkEmpty, { props: { title: 'No tasks', hint: 'Start one' } })
    expect(empty.text()).toContain('No tasks')
    expect(empty.text()).toContain('Start one')
  })
})