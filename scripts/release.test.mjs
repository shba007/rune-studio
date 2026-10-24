import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, writeFile, realpath, rm } from 'node:fs/promises'
import { join, dirname, basename, resolve } from 'node:path'
import { tmpdir } from 'node:os'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { generate, assemble, extract, applyPlan, plan, eligibility } from './release.mjs'
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const raw = (message, body = '', shortHash = 'abc1234') => ({
  message,
  body,
  shortHash,
  author: { name: 'Example Contributor', email: 'example@example.invalid' },
})
const old =
  '# Changelog\r\n\r\n## v0.0.27\r\n\r\n- Published notes.\r\n\r\n## v0.0.26\r\n\r\n- Historical Cycle 2 notes.\r\n'
function git(args) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8', windowsHide: true })
  assert.equal(result.status, 0, result.stderr)
  return result.stdout.trim()
}
async function fixture(job) {
  const directory = await mkdtemp(join(tmpdir(), 'studio-release-fixture-'))
  try {
    await mkdir(join(directory, 'src-tauri'))
    await writeFile(
      join(directory, 'package.json'),
      JSON.stringify({ name: 'fixture', version: '0.0.27', private: true }, null, 2) + '\n',
    )
    await writeFile(
      join(directory, 'src-tauri/Cargo.toml'),
      '[package]\nname = "rune-studio"\nversion = "0.0.27"\n',
    )
    await writeFile(
      join(directory, 'src-tauri/Cargo.lock'),
      'version = 4\n[[package]]\nname = "rune-studio"\nversion = "0.0.27"\n',
    )
    await writeFile(join(directory, 'src-tauri/tauri.conf.json'), '{"version":"../package.json"}')
    await writeFile(join(directory, 'CHANGELOG.md'), old)
    // Read-only access to existing repository objects; tests create no Git commits,
    // branches, tags or index changes.
    await writeFile(
      join(directory, '.git'),
      `gitdir: ${git(['rev-parse', '--absolute-git-dir'])}\n`,
    )
    await job(directory)
  } finally {
    const actual = await realpath(directory)
    assert.equal(dirname(actual).toLowerCase(), (await realpath(tmpdir())).toLowerCase())
    assert.ok(basename(actual).startsWith('studio-release-fixture-'))
    await rm(actual, { recursive: true })
  }
}
test('package policy handles features, fixes, maintenance and breaking precedence', async () => {
  for (const [base, message, body, expected] of [
    ['0.2.3', 'feat: add a capability', '', '0.3.0'],
    ['0.2.3', 'fix: repair a failure', '', '0.2.4'],
    ['0.2.3', 'perf: reduce latency', '', '0.2.4'],
    ['0.2.3', 'feat!: change the transport', '', '0.3.0'],
    ['0.2.3', 'fix: change compatibility', 'BREAKING CHANGE: use the new endpoint', '0.3.0'],
    ['1.2.3', 'feat: add a capability', '', '1.3.0'],
    ['1.2.3', 'feat!: change transport', '', '2.0.0'],
    ['1.2.3', 'chore: maintain files', 'BREAKING CHANGE: new installation layout', '2.0.0'],
    ...['docs', 'refactor', 'style', 'test', 'build', 'ci', 'chore', 'revert', 'custom'].map(
      (type) => ['0.2.3', `${type}: improve behaviour`, '', '0.2.4'],
    ),
  ]) {
    const result = await generate([raw(message, body)], base)
    assert.equal(result.version, expected, message)
  }
  const mixed = await generate(
    [
      raw('fix: correct a failure'),
      raw('feat: add a feature'),
      raw('chore!: require compatibility'),
    ],
    '1.2.3',
  )
  assert.equal(mixed.version, '2.0.0')
  for (const base of ['bad', '1.2', 'v1.2.3', '1.2.3+metadata'])
    await assert.rejects(generate([raw('fix: repair')], base))
})
test('the pinned package renders links, groups, explicit breaks and contributors deterministically', async () => {
  const commits = [
    raw('feat(insight)!: explain a selected window (#42)', '', 'abc1111'),
    raw('fix(kit): describe a failed connection', '', 'abc2222'),
  ]
  const first = await generate(commits, '0.0.27')
  const second = await generate(commits, '0.0.27')
  assert.equal(first.section, second.section)
  assert.equal(first.version, '0.1.0')
  assert.match(first.section, /## v0\.1\.0/)
  assert.match(first.section, /v0\.0\.27\.\.\.v0\.1\.0/)
  assert.match(first.section, /Enhancements/)
  assert.match(first.section, /Fixes/)
  assert.match(first.section, /Breaking Changes/)
  assert.match(first.section, /pull\/42/)
  assert.match(first.section, /commit\/abc2222/)
  assert.match(first.section, /Contributors/)
  assert.match(first.section, /Example Contributor/)
  assert.doesNotMatch(first.section, /Documentation/)
  const changelog = assemble(old, '0.0.27', '0.0.27', first)
  assert.ok(changelog.endsWith(old.slice(old.indexOf('## v0.0.27'))))
  assert.equal(await extract(changelog, first.version), first.notes)
  assert.throws(
    () => assemble(old, '0.0.27', '0.0.27', { ...first, version: '0.0.26' }),
    /decreasing/,
  )
})
test('routine bookkeeping and internal IDs are excluded while substantive maintenance remains', async () => {
  assert.equal(
    await generate(
      [
        raw('ci(release): update version'),
        raw('chore: apply code fixes [skip ci]'),
        raw('feat: finish Cycle 4'),
        raw('fix: complete INF-7 checks'),
        raw('Merge pull request #12 from release/update-version'),
        raw('not conventional'),
      ],
      '0.0.27',
    ),
    null,
  )
  assert.match(
    (await generate([raw('fix(release): retain exact generated notes')], '0.0.27')).notes,
    /Retain exact generated notes/,
  )
})
test('preparation repeats without a bump, no-op merges stay no-op, and normal main titles need no marker', async () => {
  await fixture(async (directory) => {
    const generated = await generate([raw('feat: explain a selected screen')], '0.0.27')
    const prepared = {
      ...generated,
      changelog: assemble(old, '0.0.27', '0.0.27', generated),
      baseTag: 'v0.0.27',
      sourceHead: git(['rev-parse', 'HEAD']),
    }
    await applyPlan(directory, prepared)
    const before = await readFile(join(directory, 'CHANGELOG.md'), 'utf8')
    await applyPlan(directory, prepared)
    assert.equal(await readFile(join(directory, 'CHANGELOG.md'), 'utf8'), before)
    assert.equal(await plan(directory), null)
    const release = await eligibility(directory)
    assert.equal(release.publish, true)
    assert.equal(release.notes, generated.notes)
    const merged = await eligibility(directory, async () => [
      raw('Merge pull request #55 from develop'),
    ])
    assert.equal(merged.publish, true)
    await assert.rejects(
      eligibility(directory, async () => [raw('feat: source changed after preparation')]),
      /stale/,
    )
    await writeFile(
      join(directory, 'CHANGELOG.md'),
      before.replace('Explain a selected screen', 'Different release text'),
    )
    await assert.rejects(eligibility(directory), /digest mismatch/)
  })
  await fixture(async (directory) => assert.equal((await eligibility(directory)).publish, false))
  const workflow = await readFile(join(root, '.github/workflows/release.yml'), 'utf8')
  assert.doesNotMatch(workflow, /head_commit|contains\(.*ci\(release\)/)
})
test('new source advances a pending release using the greatest package bump without duplicate sections', async () => {
  await fixture(async (directory) => {
    const sourceHead = git(['rev-parse', 'HEAD'])
    const fix = await generate([raw('fix: report connection failure')], '0.0.27')
    const first = {
      ...fix,
      changelog: assemble(old, '0.0.27', '0.0.27', fix),
      baseTag: 'v0.0.27',
      sourceHead,
    }
    await assert.rejects(applyPlan(directory, { ...first, version: '0.0.27' }), /unpublished/)
    await applyPlan(directory, first)
    const feature = await generate(
      [raw('fix: report connection failure'), raw('feat: explain a selected window')],
      '0.0.27',
    )
    assert.equal(feature.version, '0.1.0')
    const changelog = assemble(first.changelog, fix.version, '0.0.27', feature)
    assert.ok(changelog.endsWith(old.slice(old.indexOf('## v0.0.27'))))
    assert.doesNotMatch(changelog, /^## v0\.0\.28$/m)
    assert.equal((changelog.match(/^## v0\.1\.0$/gm) || []).length, 1)
    await applyPlan(directory, { ...feature, changelog, baseTag: 'v0.0.27', sourceHead })
    assert.equal(await plan(directory), null)
  })
})