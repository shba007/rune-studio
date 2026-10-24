// Package adapter and release-state orchestration. No custom bump algorithm or renderer.
import { readFile, writeFile, mkdir, mkdtemp, rm, realpath } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { join, resolve, dirname, basename } from 'node:path'
import { tmpdir } from 'node:os'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'
import semver from 'semver'
import defaults, { releasePolicy } from '../changelog.config.mjs'
const home = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const GENERATOR = 'changelogen@0.6.2'
const sha = (value) => createHash('sha256').update(value).digest('hex')
const privateLabel =
  /\b(?:cycle|milestone)\s*[-#]?\s*\d+\b|\b(?:M|K|L|R|INF|KF|WP)[- ]?\d+\b|\b(?:TODO|TBD|placeholder)\b/i
let packagePromise
async function generator() {
  // changelogen's optional contributor lookup is resolved offline from reviewed
  // configuration. Package rendering still owns the Contributors section.
  if (!packagePromise) {
    if (process.env.FORCE_NODE_FETCH)
      throw new Error(
        'Unset FORCE_NODE_FETCH: release generation requires the deterministic offline author adapter',
      )
    globalThis.fetch = async (input) => {
      const url = new URL(typeof input === 'string' ? input : input.url)
      if (url.origin !== 'https://ungh.cc' || !url.pathname.startsWith('/users/find/'))
        throw new Error('Release generation does not use network services')
      const email = decodeURIComponent(url.pathname.slice('/users/find/'.length))
      const handle = releasePolicy.authorHandles[email]
      return new Response(JSON.stringify({ user: handle ? { username: handle } : null }), {
        headers: { 'content-type': 'application/json' },
      })
    }
    packagePromise = import('changelogen')
  }
  return packagePromise
}
function run(command, args, cwd = home) {
  const result = spawnSync(command, args, { cwd, encoding: 'utf8', windowsHide: true })
  if (result.error) throw result.error
  if (result.status !== 0)
    throw new Error(`${command} ${args[0]} failed: ${result.stderr || result.stdout}`)
  return result.stdout
}
function rust(args, root = home) {
  return run('cargo', [
    'run',
    '--quiet',
    '--locked',
    '--manifest-path',
    join(home, 'xtask/Cargo.toml'),
    '--',
    ...args,
    '--root',
    root,
  ])
}
function git(root, args) {
  return run('git', args, root).trim()
}
function version(value) {
  if (semver.valid(value) !== value || semver.prerelease(value) || value.includes('+'))
    throw new Error(`Invalid stable release version: ${value}`)
  return value
}
async function cleanupScratch(directory) {
  const actual = await realpath(directory)
  if (
    dirname(actual).toLowerCase() !== (await realpath(tmpdir())).toLowerCase() ||
    !basename(actual).startsWith('studio-changelogen-')
  )
    throw new Error('Refusing to remove an unexpected temporary path')
  await rm(actual, { recursive: true })
}
async function scratch(job) {
  const directory = await mkdtemp(join(tmpdir(), 'studio-changelogen-'))
  try {
    return await job(directory)
  } finally {
    await cleanupScratch(directory)
  }
}
export async function parsedCommits(raw, baseline) {
  const pkg = await generator()
  const types = structuredClone(defaults.types)
  // changelogen 0.6.2 maps a pre-1.0 major to minor, and minor to patch.
  // Explicitly configure pre-1.0 features as package-major, producing policy-minor.
  const preOne = semver.major(version(baseline)) === 0
  if (releasePolicy.preOneBreaking !== 'minor' || releasePolicy.stableBreaking !== 'major')
    throw new Error('Pinned changelogen supports the agreed breaking-change policy only')
  const feature = preOne ? releasePolicy.preOneFeatures : releasePolicy.stableFeatures
  types.feat.semver = preOne && feature === 'minor' ? 'major' : feature
  const config = await pkg.loadChangelogConfig(home, {
    ...defaults,
    types,
    from: `v${baseline}`,
    to: 'HEAD',
    output: false,
    tokens: {},
  })
  const parsed = pkg.parseCommits(raw, config).filter((commit) => {
    const description = commit.description
    return (
      !/^Merge\b/i.test(commit.message) &&
      !(['ci', 'chore'].includes(commit.type) && commit.scope === 'release') &&
      !(
        ['ci', 'chore'].includes(commit.type) &&
        /^(?:update|bump) version\b|^apply code fixes(?:\s|$)/i.test(description)
      ) &&
      !privateLabel.test(`${commit.scope} ${description}`)
    )
  })
  for (const commit of parsed) {
    if (!config.types[commit.type])
      config.types[commit.type] = {
        title: commit.type[0].toUpperCase() + commit.type.slice(1),
        semver: releasePolicy.maintenance,
      }
    if (!config.types[commit.type].semver)
      config.types[commit.type].semver = releasePolicy.maintenance
  }
  return { pkg, config, commits: parsed }
}
export async function generate(raw, baseline) {
  const { pkg, config, commits } = await parsedCommits(raw, baseline)
  if (!commits.length) return null
  return scratch(async (directory) => {
    await writeFile(
      join(directory, 'package.json'),
      JSON.stringify({ name: 'release-calculation-fixture', version: version(baseline) }),
    )
    const calculation = { ...config, cwd: directory }
    const next = await pkg.bumpVersion(commits, calculation)
    if (!next || !semver.gt(version(next), baseline))
      throw new Error('Generator did not produce an increasing release version')
    const section = await pkg.generateMarkDown(commits, { ...config, newVersion: next })
    const file = join(directory, 'CHANGELOG.md')
    await writeFile(file, `${section}\n`)
    const notes = rust(['extract', file, next])
    return {
      version: next,
      section,
      notes,
      notesHash: sha(notes),
      commits: commits.map((commit) => commit.shortHash),
    }
  })
}
export async function extract(changelog, next) {
  return scratch(async (directory) => {
    const file = join(directory, 'CHANGELOG.md')
    await writeFile(file, changelog)
    return rust(['extract', file, version(next)])
  })
}
export function assemble(history, current, baseline, generated) {
  // Only replace an unpublished generated section. The historical suffix remains byte-for-byte.
  const headings = [...history.matchAll(/^## v?(\d+\.\d+\.\d+)\r?$/gm)]
  const first = headings[0]
  if (!first || first[1] !== current)
    throw new Error('First changelog heading must match the current package version')
  const pending = semver.gt(current, baseline)
  if (semver.lt(generated.version, current))
    throw new Error('Refusing a decreasing prepared release version')
  const tail = pending
    ? history.slice(headings[1]?.index ?? history.length)
    : history.slice(first.index)
  return history.slice(0, first.index) + generated.section + '\n\n' + tail
}
async function rawHistory(root, from, to) {
  const pkg = await generator()
  git(root, ['merge-base', '--is-ancestor', from, to])
  const nonMerges = git(root, ['rev-list', '--no-merges', `${from}..${to}`]).split(/\s+/)
  return (await pkg.getGitDiff(from, to, root)).filter((commit) =>
    nonMerges.some((hash) => hash.startsWith(commit.shortHash)),
  )
}
async function snapshot(root) {
  const packageJson = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
  const current = version(packageJson.version)
  const tags = git(root, ['tag', '--list', 'v*'])
    .split(/\r?\n/)
    .filter((tag) => /^v\d+\.\d+\.\d+$/.test(tag) && semver.valid(tag.slice(1)))
  tags.sort((a, b) => semver.rcompare(a.slice(1), b.slice(1)))
  if (!tags.length) throw new Error('Fetch complete release tags before preparing a release')
  const baseTag = tags[0],
    baseline = baseTag.slice(1)
  if (semver.lt(current, baseline))
    throw new Error('Package version is older than the latest published tag')
  const head = git(root, ['rev-parse', 'HEAD'])
  const history = await readFile(join(root, 'CHANGELOG.md'), 'utf8')
  const statePath = join(root, '.release/prepared.json')
  const state = existsSync(statePath) ? JSON.parse(await readFile(statePath, 'utf8')) : null
  return { current, baseline, baseTag, head, history, state }
}
function checkState(data, notes) {
  const state = data.state
  if (
    !state ||
    state.generator !== GENERATOR ||
    state.version !== data.current ||
    state.baseTag !== data.baseTag ||
    !/^[a-f0-9]{40}$/.test(state.sourceHead) ||
    state.notesHash !== sha(notes)
  )
    throw new Error(
      'Invalid pending prepared release: regenerate it through develop CI (version/source/generator/notes digest mismatch)',
    )
}
export async function plan(root) {
  const data = await snapshot(root)
  const pending = semver.gt(data.current, data.baseline)
  if (pending) {
    const notes = rust(['notes'], root)
    checkState(data, notes)
    git(root, ['merge-base', '--is-ancestor', data.baseTag, data.state.sourceHead])
    const newRaw = await rawHistory(root, data.state.sourceHead, data.head)
    const { commits } = await parsedCommits(newRaw, data.baseline)
    if (!commits.length) return null
  }
  const generated = await generate(await rawHistory(root, data.baseTag, data.head), data.baseline)
  if (!generated) return null
  const changelog = assemble(data.history, data.current, data.baseline, generated)
  if ((await extract(changelog, generated.version)) !== generated.notes)
    throw new Error('Prepared note extraction disagrees with the package-generated section')
  return {
    ...generated,
    changelog,
    sourceHead: data.head,
    baseTag: data.baseTag,
    generator: GENERATOR,
  }
}
export async function applyPlan(root, prepared) {
  const current = JSON.parse(await readFile(join(root, 'package.json'), 'utf8')).version
  if (
    !semver.gt(version(prepared.version), prepared.baseTag.slice(1)) ||
    semver.lt(prepared.version, version(current))
  )
    throw new Error('Prepared version must be unpublished and nondecreasing')

  const state = {
    generator: GENERATOR,
    version: prepared.version,
    sourceHead: prepared.sourceHead,
    baseTag: prepared.baseTag,
    notesHash: prepared.notesHash,
  }
  // The actual section was rendered by changelogen above, never handwritten.
  rust(['sync', prepared.version], root)
  await writeFile(join(root, 'CHANGELOG.md'), prepared.changelog)
  await mkdir(join(root, '.release'), { recursive: true })
  await writeFile(join(root, '.release/prepared.json'), JSON.stringify(state, null, 2) + '\n')
  if (rust(['notes'], root) !== prepared.notes)
    throw new Error('Applied notes do not match the generated preview')
}
export async function eligibility(root, history = rawHistory) {
  const notes = rust(['notes'], root) // Validate versions/notes once, including no-op pushes.
  const data = await snapshot(root)
  if (data.current === data.baseline)
    return {
      publish: false,
      version: data.current,
      reason: 'Version already published; nothing to release.',
    }
  checkState(data, notes)
  git(root, ['merge-base', '--is-ancestor', data.baseTag, data.state.sourceHead])
  const { commits } = await parsedCommits(
    await history(root, data.state.sourceHead, data.head),
    data.baseline,
  )
  if (commits.length)
    throw new Error(
      'Prepared release is stale: new releasable source arrived after preparation. Run develop CI again.',
    )
  return { publish: true, version: data.current, notes }
}
async function outputs(values) {
  if (process.env.GITHUB_OUTPUT) {
    const { appendFile } = await import('node:fs/promises')
    for (const [key, value] of Object.entries(values))
      await appendFile(process.env.GITHUB_OUTPUT, `${key}=${value}\n`)
  }
}
async function main(args) {
  let root = home
  const at = args.indexOf('--root')
  if (at >= 0) {
    if (!args[at + 1]) throw new Error('--root requires a directory')
    root = resolve(args[at + 1])
    args.splice(at, 2)
  }
  const [command, ...options] = args
  if (command === 'prepare') {
    if (options.some((option) => option !== '--apply')) throw new Error('Usage: prepare [--apply]')
    const write = options.includes('--apply')
    if (
      write &&
      (process.env.GITHUB_ACTIONS !== 'true' || process.env.GITHUB_REF !== 'refs/heads/develop')
    )
      throw new Error(
        'Actual preparation writes are restricted to trusted develop CI; use the default read-only preview locally',
      )
    const prepared = await plan(root)
    if (!prepared) {
      console.log('No releasable changes; no preparation PR needed.')
      await outputs({ prepared: false })
      return
    }
    if (process.env.GITHUB_SHA && prepared.sourceHead !== process.env.GITHUB_SHA)
      throw new Error('Tested source revision does not match preparation HEAD')
    console.log(
      `Generated preview (working changelog unchanged unless --apply):\n\n${prepared.section}`,
    )
    if (write) {
      if (git(root, ['rev-parse', 'origin/develop']) !== prepared.sourceHead)
        throw new Error('Preparation source is stale')
      await applyPlan(root, prepared)
    }
    await outputs({ prepared: true })
  } else if (command === 'eligible') {
    if (options.length) throw new Error('Usage: eligible')
    const release = await eligibility(root)
    console.log(
      release.publish
        ? `Prepared unpublished release v${release.version} is eligible.`
        : release.reason,
    )
    await outputs({ publish: release.publish, version: release.version })
  } else if (command === 'example') {
    const generated = await generate(
      [
        {
          message: 'feat(insight): explain a selected window',
          body: '',
          shortHash: 'abcdef1',
          author: { name: 'Example Contributor', email: 'example@example.invalid' },
        },
        {
          message: 'fix(kit): report HTTP connection errors',
          body: '',
          shortHash: 'abcdef2',
          author: { name: 'Example Contributor', email: 'example@example.invalid' },
        },
      ],
      '0.0.27',
    )
    console.log(`Read-only package-generated example:\n\n${generated.section}`)
  } else throw new Error('Usage: release.mjs {prepare [--apply]|eligible|example}')
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  main(process.argv.slice(2)).catch((error) => {
    console.error(`Release preparation failed: ${error.message}`)
    process.exitCode = 1
  })