import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import StagedChanges from './StagedChanges.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))

const config = { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
const token = (n: number) => String(n).repeat(64).slice(0, 64)

function serve(opts: { applyError?: string } = {}) {
  let reviews = [
    { path: 'D:/n/a.txt', deleted: false, baseline: null, approval: token(1) },
    { path: 'D:/n/b.txt', deleted: false, baseline: 'old', approval: token(2) },
  ]
  let recoveries: { path: string; approval: string }[] = []
  api.invoke.mockReset().mockImplementation(async (command, args) => {
    if (command === 'kit_settings') return config
    if (command === 'kit_changes_inventory')
      return {
        plugins: [
          { plugin: 'rune-filesystem', reviews, recoveries },
          {
            plugin: 'rune-time',
            reviews: [{ path: 'other', deleted: false, baseline: null, approval: token(9) }],
            recoveries: [],
          },
        ],
      }
    if (command === 'kit_change_action') {
      if (opts.applyError) throw opts.applyError
      const action = args!.action as { action: string; approval: string }
      if (action.action === 'apply') {
        const done = reviews.find((item) => item.approval === action.approval)!
        reviews = reviews.filter((item) => item !== done)
        recoveries = [...recoveries, { path: done.path, approval: token(7) }]
      } else {
        recoveries = recoveries.filter((item) => item.approval !== action.approval)
      }
      return null
    }
  })
  return mount(StagedChanges, { props: { tool: 'kit__rune-filesystem__write_file' } })
}
const button = (wrapper: ReturnType<typeof serve>, text: string) =>
  wrapper.findAll('button').find((item) => item.text() === text)!
const actions = () => api.invoke.mock.calls.filter(([command]) => command === 'kit_change_action')

it("shows only this plugin's staged changes, each with what applying will do", async () => {
  const wrapper = serve()
  await flushPromises()
  expect(wrapper.text()).toContain('D:/n/a.txt')
  expect(wrapper.text()).toContain('Creates this file. Not applied yet.')
  expect(wrapper.text()).toContain('Replaces this file. Not applied yet.')
  expect(wrapper.text()).not.toContain('other')
  wrapper.unmount()
})

it('applies a change with one click, then offers Undo for exactly that file', async () => {
  const wrapper = serve()
  await flushPromises()
  await button(wrapper, 'Apply').trigger('click')
  await flushPromises()
  expect(actions()).toEqual([
    [
      'kit_change_action',
      { config, action: { plugin: 'rune-filesystem', action: 'apply', approval: token(1) } },
    ],
  ])
  expect(wrapper.text()).toContain('Applied.')
  expect(wrapper.text()).toContain('D:/n/a.txt applied')
  expect(wrapper.text()).not.toContain('D:/n/a.txt\n')
  await button(wrapper, 'Undo').trigger('click')
  await flushPromises()
  expect(actions().at(-1)![1]).toEqual({
    config,
    action: { plugin: 'rune-filesystem', action: 'restore', approval: token(7) },
  })
  expect(wrapper.text()).toContain('Undone.')
  expect(button(wrapper, 'Undo')).toBeUndefined()
  wrapper.unmount()
})

it('applies every staged change from one button', async () => {
  const wrapper = serve()
  await flushPromises()
  await wrapper.get('[data-testid="apply-all"]').trigger('click')
  await flushPromises()
  expect(actions().map(([, args]) => (args!.action as { approval: string }).approval)).toEqual([
    token(1),
    token(2),
  ])
  expect(wrapper.findAll('button').filter((item) => item.text() === 'Undo')).toHaveLength(2)
  wrapper.unmount()
})

it('reports a refused apply, keeps the change pending and offers Reload', async () => {
  const wrapper = serve({ applyError: 'The reviewed item changed or no longer exists' })
  await flushPromises()
  await button(wrapper, 'Apply').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('changed or no longer exists')
  expect(wrapper.text()).toContain('D:/n/a.txt')
  expect(button(wrapper, 'Reload')).toBeDefined()
  wrapper.unmount()
})

it('says so when nothing is waiting', async () => {
  api.invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'kit_settings') return config
    return { plugins: [{ plugin: 'rune-filesystem', reviews: [], recoveries: [] }] }
  })
  const wrapper = mount(StagedChanges, { props: { tool: 'rune-filesystem__write_file' } })
  await flushPromises()
  expect(wrapper.text()).toContain('No staged changes waiting.')
  wrapper.unmount()
})