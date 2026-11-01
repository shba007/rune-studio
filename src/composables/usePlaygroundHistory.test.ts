import { beforeEach, describe, expect, it } from 'vite-plus/test'
import { usePlaygroundHistory, MAX_HISTORY_BYTES, MAX_HISTORY_RUNS } from './usePlaygroundHistory'
import { EMPTY_SNAPSHOT } from '@/config/analysis'

beforeEach(() => usePlaygroundHistory().clear())

describe('Playground history', () => {
  it('records completed runs, evicts the oldest by count, and clears an evicted view', () => {
    const history = usePlaygroundHistory()
    history.record('ask', { ...EMPTY_SNAPSHOT, id: 'pending', status: 'asking' })
    expect(history.runs.value).toHaveLength(0)
    for (let i = 0; i < MAX_HISTORY_RUNS; i++)
      history.recordModel('tts', { text: String(i), model: 'm', elapsedMs: 1 }, [])
    const oldest = history.runs.value.at(-1)!
    history.view(oldest.id)
    history.recordModel('tts', { text: 'new', model: 'm', elapsedMs: 1 }, [])
    expect(history.runs.value).toHaveLength(MAX_HISTORY_RUNS)
    expect(history.viewed.value).toBeNull()
    expect(history.notice.value).toContain('1 older run was removed')
    history.clear()
    expect(history.runs.value).toHaveLength(0)
    expect(history.notice.value).toBe('')
  })

  it('enforces a conservative memory limit and keeps prior runs when one result is too large', () => {
    const history = usePlaygroundHistory()
    const large = 'a'.repeat(Math.floor(MAX_HISTORY_BYTES / 4))
    history.recordModel('tts', { text: large, model: 'm', elapsedMs: 1 }, [])
    history.recordModel('tts', { text: large, model: 'm', elapsedMs: 1 }, [])
    expect(history.runs.value).toHaveLength(1)
    expect(history.notice.value).toContain('removed')
    const previous = history.runs.value[0]!.id
    history.recordModel(
      'tts',
      { text: 'x'.repeat(MAX_HISTORY_BYTES / 2), model: 'm', elapsedMs: 1 },
      [],
    )
    expect(history.runs.value[0]!.id).toBe(previous)
    expect(history.notice.value).toContain('too large')
  })
})