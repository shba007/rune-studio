import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import PersonalitiesPanel from './PersonalitiesPanel.vue'
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
it('requires explicit approval to save, sends the expected version, and preserves conflicted edits', async () => {
  const base = {
    id: 'base',
    name: 'Rune',
    mark: 'R',
    categories: [],
    prompt: 'Clear and supportive',
  }
  invoke.mockImplementation(async (command) => {
    if (command === 'personalities_state')
      return {
        entries: [base],
        folder: 'Studio/prompts/personalities',
        active: { id: 'base', name: 'Rune', mark: 'R' },
      }
    throw new Error('Conflict: reload and review')
  })
  const wrapper = mount(PersonalitiesPanel)
  await flushPromises()
  await wrapper.get('textarea').setValue('Encourage steady progress')
  expect(invoke.mock.calls.filter(([command]) => command === 'personality_save')).toHaveLength(0)
  await wrapper.get('[aria-label="Edit personality"]').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('personality_save', {
    expected: base,
    profile: { ...base, mark: 'companion', prompt: 'Encourage steady progress' },
  })
  expect(wrapper.get('[role="alert"]').text()).toContain('Conflict')
  expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe(
    'Encourage steady progress',
  )
  wrapper.unmount()
})

it('shows an unsaved model suggestion and saves it only after explicit approval', async () => {
  const proposed = {
    id: 'fitness-draft',
    name: 'Fitness coach',
    mark: 'fitness',
    categories: ['fitness'],
    prompt: 'Encourage consistent effort.',
  }
  const base = {
    id: 'base',
    name: 'Rune',
    mark: 'companion',
    categories: [],
    prompt: 'Clear and supportive',
  }
  invoke.mockReset()
  invoke.mockImplementation(async (command) => {
    if (command === 'personality_propose') return proposed
    return {
      entries: command === 'personality_save' ? [base, proposed] : [base],
      folder: 'Studio/prompts/personalities',
      active: { id: 'base', name: 'Rune', mark: 'companion' },
    }
  })
  const wrapper = mount(PersonalitiesPanel)
  await flushPromises()
  await wrapper.get('[aria-label="Suggest a personality"] input').setValue('fitness')
  await wrapper.get('[aria-label="Suggest a personality"]').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('personality_propose', { category: 'fitness' })
  expect(invoke.mock.calls.filter(([command]) => command === 'personality_save')).toHaveLength(0)
  expect(wrapper.text()).toContain('Unsaved suggestion')
  expect(wrapper.get('select').element.value).toBe('fitness')
  await wrapper.get('textarea').setValue('My approved coaching style')
  await wrapper.get('[aria-label="Edit personality"]').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('personality_save', {
    expected: null,
    profile: { ...proposed, prompt: 'My approved coaching style' },
  })
  wrapper.unmount()
})