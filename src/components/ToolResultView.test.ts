import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import ToolResultView from './ToolResultView.vue'

it('shows time data as readable fields and keeps the complete MCP response in a closed disclosure', () => {
  const text = JSON.stringify({
    content: [
      {
        type: 'text',
        text: JSON.stringify({
          result: {
            datetime: '2026-11-17T03:10:54+05:30',
            timezone: 'Asia/Kolkata',
            utcOffset: '+05:30',
          },
          status: 'success',
        }),
      },
    ],
    resultType: 'complete',
    _meta: { server: 'rune-kit' },
  })
  const wrapper = mount(ToolResultView, { props: { text } })
  expect(wrapper.findAll('dt').map((field) => field.text())).toEqual([
    'Result',
    'Datetime',
    'Timezone',
    'Utc Offset',
    'Status',
  ])
  expect(wrapper.get('dl').text()).toContain('Asia/Kolkata')
  expect(wrapper.get('dl').text()).not.toContain('resultType')
  expect(wrapper.get('details summary').text()).toBe('Raw JSON')
  expect(wrapper.get('details').attributes('open')).toBeUndefined()
  expect(wrapper.get('details pre').text()).toBe(text)
  wrapper.unmount()
})

it('renders directory-style objects as a table with all columns and accurate missing values', () => {
  const wrapper = mount(ToolResultView, {
    props: {
      text: JSON.stringify([
        { file_name: 'notes.txt', size: 0, writable: false },
        { file_name: 'draft.txt', extra: null },
      ]),
    },
  })
  expect(wrapper.findAll('th').map((cell) => cell.text())).toEqual([
    'File name',
    'Size',
    'Writable',
    'Extra',
  ])
  const rows = wrapper.findAll('tbody tr')
  expect(rows).toHaveLength(2)
  expect(rows[0]!.findAll('td').map((cell) => cell.text())).toEqual(['notes.txt', '0', 'No', '—'])
  expect(rows[1]!.findAll('td').map((cell) => cell.text())).toEqual(['draft.txt', '—', '—', 'Null'])
  wrapper.unmount()
})

it('renders text, mixed lists and empty collections without executing markup', () => {
  const text = 'Notes\n<script>alert(1)</script>'
  const wrapper = mount(ToolResultView, { props: { text } })
  expect(wrapper.find('script').exists()).toBe(false)
  expect(wrapper.get('p').text()).toBe(text)
  wrapper.unmount()
  const lists = mount(ToolResultView, {
    props: { text: JSON.stringify({ list: [1, 'two', false], emptyList: [], emptyObject: {} }) },
  })
  expect(lists.findAll('li').map((item) => item.text())).toEqual(['1', 'two', 'No'])
  expect(lists.text()).toContain('No items')
  expect(lists.text()).toContain('No fields')
  lists.unmount()
})

it('uses schema labels and descriptions and toggles to the unchanged text', async () => {
  const text = '{"content":[{"type":"text","text":"Original response"}]}'
  const wrapper = mount(ToolResultView, {
    props: {
      text,
      schema: {
        type: 'object',
        properties: {
          count: { type: 'integer', title: 'Item count', description: 'Number of items' },
          enabled: { type: 'boolean' },
        },
      },
      structuredContent: { enabled: false, count: 3 },
    },
  })
  expect(wrapper.findAll('dt').map((field) => field.text())).toEqual(['Item count', 'Enabled'])
  expect(wrapper.get('dt').attributes('title')).toBe('Number of items')
  expect(wrapper.findAll('dd').map((value) => value.text())).toEqual(['3', 'No'])
  await wrapper.get('button').trigger('click')
  expect(wrapper.get('button').attributes('aria-pressed')).toBe('true')
  expect(wrapper.get('pre').text()).toBe(text)
  await wrapper.get('button').trigger('click')
  expect(wrapper.find('dl').exists()).toBe(true)
  wrapper.unmount()
})

it('keeps the existing text view when either schema or structured content is absent', () => {
  for (const extra of [
    { schema: { type: 'object' } },
    { structuredContent: { ignored: 'not schema-driven' } },
  ]) {
    const wrapper = mount(ToolResultView, { props: { text: 'plain text as before', ...extra } })
    expect(wrapper.get('p').text()).toBe('plain text as before')
    expect(wrapper.find('button').exists()).toBe(false)
    expect(wrapper.get('details pre').text()).toBe('plain text as before')
    wrapper.unmount()
  }
})