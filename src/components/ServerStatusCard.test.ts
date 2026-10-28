import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'

import type { Agent } from '@/composables/useAgent'

import ServerStatusCard from './ServerStatusCard.vue'

function show(tone: 'down' | 'warn', title = 'Model server offline') {
  const refreshBackend = vi.fn<() => Promise<void>>(async () => {})
  const openSettings = vi.fn<(section?: string) => Promise<void>>(async () => {})
  const agent = {
    health: ref({ tone, title }),
    settings: ref({ baseUrl: 'http://127.0.0.1:8080' }),
    refreshBackend,
    openSettings,
  } as unknown as Agent
  return { wrapper: mount(ServerStatusCard, { props: { agent } }), refreshBackend, openSettings }
}

it('explains an unreachable server with its address and offers both ways out', async () => {
  const { wrapper, refreshBackend, openSettings } = show('down')
  expect(wrapper.text()).toContain('Model server not reachable')
  expect(wrapper.get('[data-testid="server-address"]').text()).toBe('http://127.0.0.1:8080')
  await wrapper.findAll('button')[0]!.trigger('click')
  await flushPromises()
  expect(refreshBackend).toHaveBeenCalledOnce()
  await wrapper.findAll('button')[1]!.trigger('click')
  expect(openSettings).toHaveBeenCalledWith('models')
})

it('shows the health reason when the server answers but is misconfigured', () => {
  const { wrapper } = show('warn', 'API key rejected')
  expect(wrapper.text()).toContain('Model server needs attention')
  expect(wrapper.text()).toContain('API key rejected')
  expect(wrapper.text()).not.toContain('Start your model server')
})