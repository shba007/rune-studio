import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import KnowledgeOverview from './KnowledgeOverview.vue'
import { knowledgeConcepts, knowledgeLinks } from '@/config/knowledge'

it('explains every concept, marks planned work honestly, and links to its workspace', async () => {
  const wrapper = mount(KnowledgeOverview)
  const pick = async (id: string) => {
    await wrapper.get('button[aria-haspopup="listbox"]').trigger('click')
    const title = knowledgeConcepts.find((node) => node.id === id)!.title
    await wrapper
      .findAll('[role="option"]')
      .find((option) => option.text().startsWith(title))!
      .trigger('click')
  }
  expect(knowledgeConcepts.find((node) => node.id === 'goals')).toMatchObject({ x: 450, y: 350 })
  expect(knowledgeLinks).toContainEqual([
    'goals',
    'personalities',
    'uses an approved communication style',
  ])
  expect(wrapper.findAll('svg [role="button"]')).toHaveLength(knowledgeConcepts.length)
  await wrapper.get('button[aria-haspopup="listbox"]').trigger('click')
  expect(wrapper.findAll('[role="option"]')).toHaveLength(knowledgeConcepts.length)
  await wrapper.get('button[aria-haspopup="listbox"]').trigger('click')
  await wrapper.get('[aria-label="Goals: Partial"]').trigger('keydown', { key: 'Enter' })
  expect(wrapper.get('[aria-label="Concept explanation"]').text()).toContain(
    'Protected storage requires opt-in',
  )
  const openGoals = wrapper.findAll('button').find((button) => button.text() === 'Open Goals')!
  await openGoals.trigger('click')
  expect(wrapper.emitted('navigate')).toEqual([['goals']])
  await pick('pathways')
  expect(wrapper.get('[aria-label="Concept explanation"]').text()).toContain(
    'proactive coaching remain future work',
  )
  expect(wrapper.get('[aria-label="Concept explanation"]').text()).toContain(
    'Reminders require your action',
  )
  await pick('consent')
  expect(wrapper.get('[aria-label="Concept explanation"]').text()).toContain(
    'do not assume all are encrypted',
  )
  wrapper.unmount()
})

it('zooms, pans with the keyboard, resets, and keeps concept selection intact', async () => {
  const wrapper = mount(KnowledgeOverview)
  await wrapper.get('[aria-label="Zoom in"]').trigger('click')
  expect(wrapper.text()).toContain('110%')
  const graph = wrapper.get('svg')
  await graph.trigger('keydown', { key: 'ArrowRight' })
  expect(graph.html()).toContain('translate(475 350) scale(1.1)')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Reset view')!
    .trigger('click')
  expect(graph.html()).toContain('translate(450 350) scale(1)')
  expect(wrapper.get('[aria-label="Goals: Partial"]').attributes('aria-pressed')).toBe('true')
  wrapper.unmount()
})