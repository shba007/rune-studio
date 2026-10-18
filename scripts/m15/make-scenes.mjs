// Renders the M1.5 probe scenes: one PNG (what the model will see) and one JSON (where every
// target really is) per scene, using headless Chrome with a throwaway profile.
//
//   node scripts/m15/make-scenes.mjs [outDir]        default: temp/m15/scenes (git-ignored)

import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, writeFileSync, existsSync, statSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { SCENES, renderScene } from './scenes.mjs'

const CHROME = [
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
].find(existsSync)
if (!CHROME)
  throw new Error('Chrome is not installed; the probe renders its scenes with headless Chrome')

const outDir = resolve(process.argv[2] ?? 'temp/m15/scenes')
mkdirSync(outDir, { recursive: true })
const profile = mkdtempSync(join(tmpdir(), 'rune-m15-chrome-'))

const flags = (extra) => [
  '--headless=new',
  '--disable-gpu',
  '--hide-scrollbars',
  '--force-device-scale-factor=1',
  '--no-first-run',
  `--user-data-dir=${profile}`,
  '--window-size=1920,1080',
  ...extra,
]
const run = (extra) =>
  spawnSync(CHROME, flags(extra), {
    encoding: 'utf8',
    timeout: 90_000,
    maxBuffer: 64 * 1024 * 1024,
  })

const sizeClass = (t) =>
  Math.min(t.w, t.h) < 24 ? 'small' : Math.min(t.w, t.h) < 44 ? 'medium' : 'large'
const index = []

for (const scene of SCENES) {
  const htmlPath = join(outDir, `${scene.id}.html`)
  const pngPath = join(outDir, `${scene.id}.png`)
  writeFileSync(htmlPath, renderScene(scene))
  const url = pathToFileURL(htmlPath).href

  const dom = run(['--virtual-time-budget=2000', '--dump-dom', url])
  const match = /<pre id="truth"[^>]*>([\s\S]*?)<\/pre>/.exec(dom.stdout ?? '')
  if (!match) throw new Error(`${scene.id}: could not read the ground truth`)
  const truth = JSON.parse(
    match[1]
      .replace(/&quot;/g, '"')
      .replace(/&amp;/g, '&')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>'),
  )

  const shot = run([`--screenshot=${pngPath}`, url])
  if (!existsSync(pngPath) || statSync(pngPath).size < 5000)
    throw new Error(`${scene.id}: no screenshot (${shot.stderr?.slice(0, 200)})`)

  // Keep only targets that are fully on stage, have a unique name, and are really there.
  const seen = new Map()
  for (const t of truth.targets) seen.set(t.name, (seen.get(t.name) ?? 0) + 1)
  const usable = truth.targets
    .filter(
      (t) => t.w > 2 && t.h > 2 && t.l >= 0 && t.t >= 0 && t.l + t.w <= 1920 && t.t + t.h <= 1080,
    )
    .filter((t) => seen.get(t.name) === 1)
    .map((t) => ({ ...t, size: sizeClass(t) }))

  writeFileSync(
    join(outDir, `${scene.id}.json`),
    JSON.stringify(
      {
        scene: scene.id,
        kind: scene.kind,
        theme: scene.theme,
        width: 1920,
        height: 1080,
        targets: usable,
      },
      null,
      2,
    ),
  )
  index.push({
    scene: scene.id,
    targets: usable.length,
    sizes: usable.reduce((n, t) => ({ ...n, [t.size]: (n[t.size] ?? 0) + 1 }), {}),
  })
  console.log(
    `${scene.id.padEnd(16)} ${String(usable.length).padStart(2)} targets  ${JSON.stringify(index.at(-1).sizes)}`,
  )
}
writeFileSync(join(outDir, 'index.json'), JSON.stringify(index, null, 2))
console.log(`\n${SCENES.length} scenes in ${outDir}`)