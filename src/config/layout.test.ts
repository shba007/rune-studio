import { expect, it } from 'vite-plus/test'
import { PAGE_GAP, pillContentHeight } from './layout'

it('reserves the card-to-composer gap even when no cards are visible', () => {
  expect(pillContentHeight(0, 112)).toBe(112 + PAGE_GAP)
  expect(pillContentHeight(80, 112)).toBe(80 + 112 + PAGE_GAP)
  expect(pillContentHeight(80, 0)).toBe(80)
})