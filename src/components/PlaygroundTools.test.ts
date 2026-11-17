import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import PlaygroundTools from './PlaygroundTools.vue'
import RuneChoice from './RuneChoice.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
const config = {
  enabled: true,
  endpoint: 'http://127.0.0.1:47601/mcp',
  selected: ['rune-time__get_current_time'],
}
const schema = {
  type: 'object',
  required: ['timezone'],
  properties: {
    timezone: { type: 'string' },
    count: { type: 'integer' },
    enabled: { type: 'boolean' },
    format: { type: 'string', enum: ['short', 'long'] },
  },
}
const tool = {
  name: 'rune-time__get_current_time',
  description: 'Read the current time',
  inputSchema: schema,
}
beforeEach(() => {
  harness.invoke.mockReset()
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'kit_settings') return config
    if (command === 'kit_discover')
      return {
        tools: [tool, { ...tool, name: 'rune-time__convert_time' }],
        blocked: [
          {
            name: 'write__file',
            description: 'Write a file',
            kind: 'writes',
            reason: 'changes files or other state; the Playground runs it only after you confirm',
          },
          {
            name: 'write__other',
            description: 'Other',
            kind: 'unavailable',
            reason: 'plugin metadata is missing',
          },
        ],
      }
    if (command === 'kit_call_tool') return { text: '{"time":"12:30"}' }
    return null
  })
})

it('groups all tools, keeps descriptions in the middle, and submits selected tools with typed arguments', async () => {
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('kit_discover', { config })
  const list = wrapper.get('[aria-label="rune-time"]')
  expect(list.text()).toContain('get_current_time')
  expect(list.get('[title="rune-time__get_current_time"]').text()).not.toContain('Off')
  expect(list.text()).not.toContain(tool.description)
  expect(list.get('[title="rune-time__convert_time"]').text()).toContain('Off')
  expect(wrapper.get('[aria-label="Tool settings"]').text()).toContain(tool.description)
  expect(wrapper.get('[aria-label="Tool settings"]').find('form').exists()).toBe(true)
  expect(wrapper.get('[aria-label="Tool result"]').find('form').exists()).toBe(false)
  expect(wrapper.get('[aria-label="write"]').text()).toContain('file')
  expect(wrapper.get('[title="write__file"]').text()).toContain('Writes')
  expect(wrapper.get('[title="write__other"]').text()).toContain('Not enabled')
  expect(list.get('[title="rune-time__get_current_time"]').text()).toContain('Runnable')
  expect(wrapper.get('[aria-label="write"]').text()).not.toContain('plugin metadata')
  await wrapper.get('[aria-label="timezone"]').setValue('Asia/Kolkata')
  await wrapper.get('[aria-label="count"]').setValue('3')
  wrapper
    .findAllComponents(RuneChoice)
    .find((choice) => choice.props().label === 'enabled')!
    .vm.$emit('update:modelValue', 'false')
  wrapper
    .findAllComponents(RuneChoice)
    .find((choice) => choice.props().label === 'format')!
    .vm.$emit('update:modelValue', '1')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('kit_call_tool', {
    name: tool.name,
    args: { timezone: 'Asia/Kolkata', count: 3, enabled: false, format: 'long' },
  })
  expect(wrapper.get('[aria-label="Tool result"]').text()).toContain('Succeeded')
  expect(wrapper.get('[aria-label="Tool result"] dt').text()).toBe('Time')
  expect(wrapper.get('[aria-label="Tool result"] dd').text()).toBe('12:30')
  expect(wrapper.get('pre').text()).toBe('{\n  "time": "12:30"\n}')
  expect(wrapper.get('[aria-label="Tool result"]').text()).toMatch(/\d+ ms/)
  expect(
    harness.invoke.mock.calls.every(([command]) =>
      ['kit_settings', 'kit_discover', 'kit_call_tool'].includes(command),
    ),
  ).toBe(true)
  wrapper.unmount()
})

it('allows inspecting not-selected and unavailable entries without running them', async () => {
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  expect(wrapper.findAll('nav h2').map((heading) => heading.text())).toEqual(['rune-time', 'write'])
  await wrapper.get('[title="rune-time__convert_time"]').trigger('click')
  expect(wrapper.get('[aria-label="Tool settings"] header').text()).toBe('rune-time__convert_time')
  expect(wrapper.get('[aria-label="Tool settings"]').text()).toContain(tool.description)
  expect(wrapper.find('form').exists()).toBe(true)
  expect(wrapper.get('button[form="tool-arguments"]').attributes('disabled')).toBeDefined()
  expect(wrapper.text()).toContain('Select this tool in Tools settings to run it')
  expect(wrapper.text()).not.toContain('No tools discovered')
  await wrapper.get('form').trigger('submit')
  expect(harness.invoke.mock.calls.some(([command]) => command === 'kit_call_tool')).toBe(false)
  await wrapper.get('[title="write__file"]').trigger('click')
  expect(wrapper.get('[aria-label="Tool settings"] header').text()).toBe('write__file')
  expect(wrapper.get('[aria-label="Tool settings"]').text()).toContain('Write a file')
  expect(wrapper.get('[aria-label="Tool settings"]').text()).toContain(
    'Writes: changes files or other state; the Playground runs it only after you confirm.',
  )
  expect(wrapper.find('form').exists()).toBe(false)
  expect(wrapper.find('button[form="tool-arguments"]').exists()).toBe(false)
  wrapper.unmount()
})

it('does not show an empty-discovery state when only unavailable entries exist', async () => {
  harness.invoke.mockImplementation(async (command) =>
    command === 'kit_settings'
      ? config
      : {
          tools: [],
          blocked: [
            {
              name: 'write__file',
              description: 'd',
              kind: 'unavailable',
              reason: 'unsupported schema',
            },
          ],
        },
  )
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  expect(wrapper.text()).not.toContain('No tools discovered')
  expect(wrapper.get('[aria-label="Tool settings"]').text()).toContain(
    'Not enabled: unsupported schema.',
  )
  wrapper.unmount()
})

it('sends a complex schema as raw JSON and shows the exact command error', async () => {
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'kit_settings') return config
    if (command === 'kit_discover')
      return {
        tools: [
          { ...tool, inputSchema: { type: 'object', properties: { values: { type: 'array' } } } },
        ],
        blocked: [],
      }
    if (command === 'kit_call_tool') throw 'Kit tool error: refused'
    return null
  })
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  await wrapper.get('textarea').setValue('{"values":[1,2]}')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('kit_call_tool', {
    name: tool.name,
    args: { values: [1, 2] },
  })
  expect(wrapper.get('[role="alert"]').text()).toBe('Kit tool error: refused')
  expect(wrapper.get('[aria-label="Tool result"]').text()).toContain('Failed')
  wrapper.unmount()
})

it('shows discovery failures and opens Tools settings without configuring Kit', async () => {
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'kit_settings') return config
    if (command === 'kit_discover') throw 'Cannot connect to Kit'
    return null
  })
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toBe('Cannot connect to Kit')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Tools settings')!
    .trigger('click')
  expect(harness.invoke).toHaveBeenCalledWith('open_settings_window', { section: 'tools' })
  expect(wrapper.find('form').exists()).toBe(false)
  wrapper.unmount()
})

it('does not discover or run tools when Kit is disabled', async () => {
  harness.invoke.mockResolvedValue({ ...config, enabled: false })
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('Kit is disabled')
  expect(harness.invoke.mock.calls.map(([command]) => command)).toEqual(['kit_settings'])
  wrapper.unmount()
})

it('marks schema tools and renders structured output with the text collapsed below', async () => {
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'kit_settings') return config
    if (command === 'kit_discover')
      return {
        tools: [
          {
            ...tool,
            outputSchema: {
              type: 'object',
              properties: { total: { type: 'integer', title: 'Total items' } },
            },
          },
        ],
        blocked: [],
      }
    if (command === 'kit_call_tool')
      return { text: 'Original text', structuredContent: { total: 4 } }
    return null
  })
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  expect(wrapper.get(`[title="${tool.name}"]`).text()).toContain('View')
  await wrapper.get('[aria-label="timezone"]').setValue('UTC')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  const result = wrapper.get('[aria-label="Tool result"]')
  expect(result.get('dt').text()).toBe('Total items')
  expect(result.get('dd').text()).toBe('4')
  expect(result.get('details pre').text()).toBe('Original text')
  wrapper.unmount()
})
function serveWriter() {
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'kit_settings') return config
    if (command === 'kit_discover')
      return {
        tools: [tool],
        blocked: [
          {
            name: 'rune-filesystem__write_file',
            description: 'Write a file',
            kind: 'writes',
            reason: 'changes files or other state',
            inputSchema: {
              type: 'object',
              required: ['path', 'content'],
              properties: { path: { type: 'string' }, content: { type: 'string' } },
            },
          },
        ],
      }
    if (command === 'kit_call_write_tool') return { text: '{"ok":true}' }
    return null
  })
}
const writeCalls = () =>
  harness.invoke.mock.calls.filter(([command]) => command === 'kit_call_write_tool')

it('shows a write tool with a form, and nothing runs until the change is confirmed', async () => {
  serveWriter()
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  await wrapper.get('[title="rune-filesystem__write_file"]').trigger('click')
  await wrapper.get('[aria-label="path"]').setValue('notes/a.txt')
  await wrapper.get('[aria-label="content"]').setValue('hello')
  expect(wrapper.get('button[form="tool-arguments"]').text()).toBe('Review change')
  await wrapper.get('form').trigger('submit')
  const card = wrapper.get('[role="alertdialog"]')
  expect(card.text()).toContain('changes the real folder now')
  expect(card.text()).toContain('"path": "notes/a.txt"')
  expect(card.text()).toContain('"content": "hello"')
  expect(writeCalls()).toHaveLength(0)
  expect(harness.invoke.mock.calls.some(([command]) => command === 'kit_call_tool')).toBe(false)
  wrapper.unmount()
})

it('cancel leaves the folder alone and confirm runs exactly the reviewed arguments', async () => {
  serveWriter()
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  await wrapper.get('[title="rune-filesystem__write_file"]').trigger('click')
  await wrapper.get('[aria-label="path"]').setValue('a.txt')
  await wrapper.get('[aria-label="content"]').setValue('one')
  await wrapper.get('form').trigger('submit')
  const buttons = () => wrapper.get('[role="alertdialog"]').findAll('button')
  await buttons()
    .find((item) => item.text() === 'Cancel')!
    .trigger('click')
  expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
  expect(writeCalls()).toHaveLength(0)
  await wrapper.get('form').trigger('submit')
  await wrapper.get('[aria-label="content"]').setValue('two, edited after review')
  await buttons()
    .find((item) => item.text() === 'Confirm and run')!
    .trigger('click')
  await flushPromises()
  expect(writeCalls()).toEqual([
    [
      'kit_call_write_tool',
      {
        name: 'rune-filesystem__write_file',
        args: { path: 'a.txt', content: 'one' },
        confirmed: true,
      },
    ],
  ])
  expect(wrapper.get('[aria-label="Tool result"]').text()).toContain('Succeeded')
  wrapper.unmount()
})

it('closes the confirm card when another tool is chosen', async () => {
  serveWriter()
  const wrapper = mount(PlaygroundTools)
  await flushPromises()
  await wrapper.get('[title="rune-filesystem__write_file"]').trigger('click')
  await wrapper.get('[aria-label="path"]').setValue('a.txt')
  await wrapper.get('[aria-label="content"]').setValue('x')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.find('[role="alertdialog"]').exists()).toBe(true)
  await wrapper.get('[title="rune-time__get_current_time"]').trigger('click')
  expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
  wrapper.unmount()
})