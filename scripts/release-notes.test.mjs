import { test } from 'node:test'
import assert from 'node:assert/strict'
import { releaseNotes } from './release-notes.mjs'
test('extracts exact latest notes', () => {
  assert.equal(
    releaseNotes(
      '# Changelog\n\n## v0.0.23\n\n### Fixes\n\n- Discover installed Kit tools.\n\n## v0.0.22\n\n- Old notes.\n',
      '0.0.23',
    ),
    '### Fixes\n\n- Discover installed Kit tools.\n',
  )
})
test('rejects missing, mismatched, duplicate, empty and placeholder notes', () => {
  for (const text of [
    '## v0.0.22\n- Old.',
    '## v0.0.24\n- New.\n## v0.0.23\n- Older.',
    '## v0.0.23\n\n',
    '## v0.0.23\nTODO',
    '## v0.0.23\n- Cycle 3 integration',
    '## v0.0.23\n- ci(release): update version',
    '## v0.0.23\n- Valid.\n## v0.0.23\n- Duplicate.',
  ])
    assert.throws(() => releaseNotes(text, '0.0.23'))
})