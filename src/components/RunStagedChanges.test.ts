import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import type { LiveStep } from '@/config/liveSteps'
import RunStagedChanges from './RunStagedChanges.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))

const step = (over: Partial<LiveStep>): LiveStep => ({
  taskId: 1,
  step: 1,
  kind: 'tool',
  model: null,
  tool: 'kit__rune-filesystem__write_file',
  text: '',
  detail: '',
  ok: true,
  ...over,
})
const staged = { text: 'ok', structuredContent: { content: 'wrote', staged: true } }

function serve() {
  api.invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'kit_settings') return { enabled: true, endpoint: 'e', selected: [] }
    return {
      plugins: [
        {
          plugin: 'rune-filesystem',
          reviews: [
            { path: 'D:/n/notes.md', deleted: false, baseline: null, approval: '1'.repeat(64) },
          ],
          recoveries: [],
        },
      ],
    }
  })
}

it('shows the changes a run staged at the level of the run, with Apply', async () => {
  serve()
  const wrapper = mount(RunStagedChanges, { props: { steps: [step({ result: staged })] } })
  await flushPromises()
  expect(wrapper.get('[aria-label="Changes waiting for your OK"]').text()).toContain(
    'Changes waiting for your OK',
  )
  expect(wrapper.text()).toContain('D:/n/notes.md')
  expect(wrapper.findAll('button').some((item) => item.text() === 'Apply')).toBe(true)
  wrapper.unmount()
})

it('shows nothing when no step staged a change', async () => {
  serve()
  const wrapper = mount(RunStagedChanges, {
    props: {
      steps: [
        step({ result: { text: 'ok', structuredContent: { count: 1 } } }),
        step({ kind: 'screen' }),
      ],
    },
  })
  await flushPromises()
  expect(wrapper.find('[aria-label="Changes waiting for your OK"]').exists()).toBe(false)
  expect(api.invoke).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('lists the change again when a later step stages more', async () => {
  serve()
  const wrapper = mount(RunStagedChanges, { props: { steps: [step({ result: staged })] } })
  await flushPromises()
  const before = api.invoke.mock.calls.filter(([c]) => c === 'kit_changes_inventory').length
  await wrapper.setProps({ steps: [step({ result: staged }), step({ step: 2, result: staged })] })
  await flushPromises()
  const after = api.invoke.mock.calls.filter(([c]) => c === 'kit_changes_inventory').length
  expect(after).toBeGreaterThan(before)
  wrapper.unmount()
})