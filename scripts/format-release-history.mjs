// Owner-authorized formatting migration; never changes versions, tags or Git.
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { resolve, dirname, join, relative, isAbsolute } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { formatHistoricalRelease } from './release.mjs'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const hash = (text) => createHash('sha256').update(text).digest('hex')
const args = process.argv.slice(2)
const write = args.includes('--write-changelog')
const inputAt = args.indexOf('--releases')
if (inputAt < 0 || !args[inputAt + 1]) throw new Error('Expected --releases SNAPSHOT.json')
const directory = join(root, 'temp/studio-release-format')
const sourceAt = args.indexOf('--source')
const source = sourceAt < 0 ? join(root, 'CHANGELOG.md') : resolve(args[sourceAt + 1])
const before = await readFile(source, 'utf8')
const imported = spawnSync(
  'cargo',
  [
    'run',
    '--quiet',
    '--locked',
    '--manifest-path',
    join(root, 'xtask/Cargo.toml'),
    '--',
    'history-import',
    source,
  ],
  { cwd: root, encoding: 'utf8', windowsHide: true },
)
if (imported.error) throw imported.error
if (imported.status !== 0) throw new Error(imported.stderr)
const history = JSON.parse(imported.stdout)
const sections = []
for (const release of history) sections.push(await formatHistoricalRelease(release))
const changelog = '# Changelog\n\n' + sections.map((release) => release.section).join('\n\n') + '\n'
const releases = JSON.parse(await readFile(resolve(args[inputAt + 1]), 'utf8'))
const entries = []
for (const release of releases) {
  if (release.draft || release.prerelease)
    throw new Error(`Unexpected non-stable release ${release.tag_name}`)
  const generated = sections.find((section) => `v${section.version}` === release.tag_name)
  if (!generated) throw new Error(`Missing local section for ${release.tag_name}`)
  const bodyPath = join(directory, `${release.tag_name}.md`)
  const child = relative(directory, bodyPath)
  if (child.startsWith('..') || isAbsolute(child)) throw new Error('Invalid release tag path')
  entries.push({
    id: release.id,
    tag: release.tag_name,
    url: release.html_url,
    name: release.name,
    targetCommitish: release.target_commitish,
    publishedAt: release.published_at,
    draft: release.draft,
    prerelease: release.prerelease,
    beforeBodyHash: hash((release.body || '').replaceAll('\r\n', '\n')),
    bodyHash: generated.notesHash,
    bodyPath: relative(root, bodyPath).replaceAll('\\', '/'),
    assets: release.assets.map(({ id, name, size, digest }) => ({ id, name, size, digest })),
  })
}
await mkdir(directory, { recursive: true })
await writeFile(join(directory, 'CHANGELOG.before.md'), before, { flag: 'wx' }).catch((error) => {
  if (error.code !== 'EEXIST') throw error
})
await writeFile(join(directory, 'CHANGELOG.generated.md'), changelog)
for (const entry of entries) {
  const generated = sections.find((section) => `v${section.version}` === entry.tag)
  await writeFile(join(root, entry.bodyPath), generated.notes)
}
await writeFile(
  join(directory, 'manifest.json'),
  JSON.stringify(
    {
      repository: 'shba007/rune-studio',
      generator: 'changelogen@0.6.2',
      beforeChangelogHash: hash(await readFile(join(directory, 'CHANGELOG.before.md'), 'utf8')),
      afterChangelogHash: hash(changelog),
      releases: entries,
    },
    null,
    2,
  ) + '\n',
)
if (write) await writeFile(join(root, 'CHANGELOG.md'), changelog)
console.log(
  `${write ? 'Formatted' : 'Prepared'} ${sections.length} unique changelog sections and ${entries.length} GitHub release bodies with changelogen 0.6.2. No GitHub changes made.`,
)