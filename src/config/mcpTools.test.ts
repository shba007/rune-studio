import { expect, it } from 'vite-plus/test'
import { isEnabled, mcpNamespace, toolsOf, withEnabled } from './mcpTools'

it('matches both full package and legacy namespaces without matching other servers', () => {
  const names = [
    'rune-time__get_current_time',
    'time__convert_time',
    'rune-filesystem__list_directory',
    'filesystem__read_text_file',
    'other-time__get_current_time',
  ]
  expect(mcpNamespace('rune-time')).toBe('time')
  expect(toolsOf('rune-time', names)).toEqual(names.slice(0, 2))
  expect(toolsOf('rune-filesystem', names)).toEqual(names.slice(2, 4))
  expect(toolsOf('other-time', names)).toEqual([names[4]])
})

it('enables and disables both naming forms while preserving unrelated selections', () => {
  const names = ['rune-time__get_current_time', 'time__convert_time', 'filesystem__read_text_file']
  for (const name of names.slice(0, 2)) expect(isEnabled('rune-time', names, [name])).toBe(true)
  expect(isEnabled('rune-time', names, [names[2]!])).toBe(false)
  expect(withEnabled('rune-time', names, [names[2]!], true)).toEqual([names[2], names[0], names[1]])
  expect(withEnabled('rune-time', names, names, false)).toEqual([names[2]])
})