import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

import { describe, expect, it } from 'vite-plus/test'

import { AGENT_BRANDS, NEUTRAL_ON_DARK, accentOnDark, brandFor } from './agents'
import { contrast, rgba } from './color'

/** Every id `bridge::identity` can emit (`identity::all_ids`); a test on the Rust side pins its length. */
const RUST_IDS = [
  'claude',
  'antigravity',
  'opencode',
  'chatgpt',
  'codex',
  'gemini',
  'cursor',
  'cline',
  'copilot',
  'windsurf',
  'external',
]

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name)
    return statSync(path).isDirectory() ? walk(path) : [path]
  })

describe('the brand registry', () => {
  it('has an entry for every id the backend can emit, and nothing else', () => {
    expect(Object.keys(AGENT_BRANDS).sort()).toEqual([...RUST_IDS].sort())
  })

  it('keeps the ids in sync with identity.rs', () => {
    const source = readFileSync('src-tauri/src/bridge/identity.rs', 'utf8')
    const missing = RUST_IDS.filter((id) => id !== 'external' && !source.includes(`"${id}",`))
    expect(missing).toEqual([])
  })

  it('only offers an accent on dark that reads there (3:1)', () => {
    const unreadable = Object.entries(AGENT_BRANDS)
      .filter(([, brand]) => brand.accentOnDark !== null)
      .filter(([, brand]) => contrast(brand.accentOnDark ?? '#000000', '#141414') < 3)
      .map(([id]) => id)
    expect(unreadable).toEqual([])
  })

  it('falls back to the neutral light colour where the brand colour is too dark', () => {
    expect(accentOnDark('cursor')).toBe(NEUTRAL_ON_DARK)
    expect(accentOnDark('cline')).toBe(NEUTRAL_ON_DARK)
    expect(accentOnDark('claude')).toBe('#D97757')
  })

  it('draws an unknown or missing id neutral', () => {
    const accents = ['nonsense', '', null, undefined].map((id) => brandFor(id).accent)
    expect(accents).toEqual([null, null, null, null])
    expect(accentOnDark(null)).toBe(NEUTRAL_ON_DARK)
  })

  it('has glyphs that are plain path data, and says where every entry came from', () => {
    const entries = Object.entries(AGENT_BRANDS)
    const badGlyphs = entries
      .filter(([, brand]) => brand.glyph !== null)
      .filter(([, brand]) => !/^[MmLlHhVvCcSsQqTtAaZz0-9eE.,\s-]+$/.test(brand.glyph ?? ''))
    expect(badGlyphs).toEqual([])
    expect(entries.filter(([, brand]) => brand.source.length < 6)).toEqual([])
  })

  it('is the only place brand colours are written down', () => {
    const accents = Object.values(AGENT_BRANDS)
      .map((brand) => brand.accent)
      .filter((accent): accent is string => accent !== null && accent !== '#000000')
      .map((accent) => accent.toLowerCase())
    const files = walk('src').filter(
      (file) => /\.(vue|ts)$/.test(file) && !/agents(\.test)?\.ts$/.test(file),
    )
    const offenders = files.filter((file) => {
      const text = readFileSync(file, 'utf8').toLowerCase()
      return accents.some((accent) => text.includes(accent))
    })
    expect(offenders).toEqual([])
  })
})

describe('colour helpers', () => {
  it('computes WCAG contrast and rgba', () => {
    expect(contrast('#ffffff', '#000000')).toBeCloseTo(21, 0)
    expect(contrast('#141414', '#141414')).toBeCloseTo(1, 5)
    expect(rgba('#D97757', 0.5)).toBe('rgba(217, 119, 87, 0.5)')
  })
})