import { expect, it } from 'vite-plus/test'
import { describeChange, pluginOf, stagedPlugins, wasStaged } from './stagedChanges'

it('finds the plugin from an agent or Playground tool name', () => {
  expect(pluginOf('kit__rune-filesystem__write_file')).toBe('rune-filesystem')
  expect(pluginOf('rune-filesystem__write_file')).toBe('rune-filesystem')
  expect(pluginOf('click')).toBe('click')
})

it('is staged only when the result says staged is exactly true', () => {
  expect(wasStaged({ content: 'x', staged: true })).toBe(true)
  for (const value of [undefined, null, 'staged', { staged: 'true' }, { staged: false }, {}, []])
    expect(wasStaged(value)).toBe(false)
})

it('describes what applying a review will do', () => {
  expect(describeChange({ deleted: true, baseline: 'a' })).toBe('Deletes this file')
  expect(describeChange({ deleted: false, baseline: 'a' })).toBe('Replaces this file')
  expect(describeChange({ deleted: false, baseline: null })).toBe('Creates this file')
})
it('lists each plugin that staged something once, counting its staged steps', () => {
  const staged = { structuredContent: { staged: true } }
  const steps = [
    { kind: 'tool', tool: 'kit__rune-filesystem__write_file', result: staged },
    {
      kind: 'tool',
      tool: 'kit__rune-filesystem__read_text_file',
      result: { structuredContent: {} },
    },
    { kind: 'tool', tool: 'kit__rune-filesystem__edit_file', result: staged },
    { kind: 'screen', tool: 'click', result: staged },
    { kind: 'tool', tool: 'kit__other__x', result: null },
  ]
  expect(stagedPlugins(steps)).toEqual([
    { plugin: 'rune-filesystem', tool: 'kit__rune-filesystem__write_file', count: 2 },
  ])
  expect(stagedPlugins([])).toEqual([])
})