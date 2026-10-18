// Synthetic GUI scenes for the M1.5 grounding probe.
//
// Each scene is a fixed 1920x1080 HTML "stage" built from a seed, so the same seed always makes the
// same scene. Elements the probe will ask the model to click carry `data-target`; their real
// positions are read back from the browser, so the ground truth is exact.
//
//   node scripts/m15/make-scenes.mjs        (renders them; see that file)

import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const PH = JSON.parse(readFileSync(require.resolve('@iconify-json/ph/icons.json'), 'utf8'))

/** Deterministic random numbers: same seed, same scene. */
export function rng(seed) {
  let a = seed >>> 0
  const next = () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
  next.int = (lo, hi) => lo + Math.floor(next() * (hi - lo + 1))
  next.pick = (list) => list[Math.floor(next() * list.length)]
  next.shuffle = (list) => {
    const copy = [...list]
    for (let i = copy.length - 1; i > 0; i--) {
      const j = Math.floor(next() * (i + 1))
      ;[copy[i], copy[j]] = [copy[j], copy[i]]
    }
    return copy
  }
  return next
}

/** Phosphor Fill glyph as inline SVG (the same set the app uses). */
const icon = (name, px = 18) => {
  const glyph = PH.icons[`${name}-fill`]
  if (!glyph) throw new Error(`unknown icon ${name}`)
  return `<svg width="${px}" height="${px}" viewBox="0 0 256 256" fill="currentColor" aria-hidden="true">${glyph.body}</svg>`
}

const esc = (text) => text.replace(/&/g, '&amp;').replace(/</g, '&lt;')

/** Marks an element as a probe target. `name` is what the prompt calls it; `role` is its kind. */
const target = (name, role, tag, attrs, inner) =>
  `<${tag} data-target data-name="${esc(name)}" data-role="${role}" ${attrs}>${inner}</${tag}>`

const THEMES = {
  light: {
    bg: '#f3f3f3',
    surface: '#ffffff',
    text: '#1b1b1b',
    muted: '#616161',
    line: '#e0e0e0',
    hover: '#eaeaea',
  },
  dark: {
    bg: '#202020',
    surface: '#2b2b2b',
    text: '#f3f3f3',
    muted: '#a0a0a0',
    line: '#3c3c3c',
    hover: '#363636',
  },
}

const baseCss = (t, accent, fontPx) => `
  *{box-sizing:border-box}
  html,body{margin:0;overflow:hidden;background:${t.bg}}
  body{width:1920px;height:1080px;position:relative;font:${fontPx}px/1.4 'Segoe UI',system-ui,sans-serif;color:${t.text}}
  .abs{position:absolute}
  button{font:inherit;color:inherit;cursor:default}
  .btn{border:1px solid ${t.line};background:${t.surface};border-radius:6px;padding:7px 18px}
  .btn.primary{background:${accent};border-color:${accent};color:#fff}
  .icon{display:inline-flex;align-items:center;justify-content:center;border:0;background:transparent;border-radius:6px;color:${t.text}}
  .card{background:${t.surface};border:1px solid ${t.line};border-radius:8px}
  .muted{color:${t.muted}}
  input[type=text]{font:inherit;color:inherit;background:${t.surface};border:1px solid ${t.line};border-radius:6px;padding:6px 10px}
  a{color:${accent};text-decoration:none}
`

function titleBar(t, title, w) {
  const winBtn = (name, svg) =>
    target(
      name,
      'window button',
      'button',
      `aria-label="${name}" style="width:46px;height:32px;border:0;background:transparent;color:${t.text}"`,
      svg,
    )
  return `<div class="abs" style="left:0;top:0;width:${w}px;height:32px;background:${t.surface};border-bottom:1px solid ${t.line}">
    <span class="abs" style="left:14px;top:6px">${esc(title)}</span>
    <div class="abs" style="right:0;top:0;display:flex">
      ${winBtn('Minimize', '<svg width="12" height="12"><rect y="6" width="10" height="1.5" fill="currentColor"/></svg>')}
      ${winBtn('Maximize', '<svg width="12" height="12"><rect x="1" y="1" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.4"/></svg>')}
      ${winBtn('Close', '<svg width="12" height="12"><path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.5"/></svg>')}
    </div></div>`
}

// --- the five layouts -------------------------------------------------------------------------

function settings(r, t, accent, f) {
  const nav = r
    .shuffle([
      ['System', 'monitor'],
      ['Bluetooth & devices', 'bluetooth'],
      ['Network & internet', 'wifi-high'],
      ['Personalization', 'paint-brush'],
      ['Apps', 'squares-four'],
      ['Accounts', 'user'],
      ['Time & language', 'clock'],
      ['Gaming', 'game-controller'],
      ['Accessibility', 'person-simple-circle'],
      ['Privacy & security', 'shield-check'],
      ['Windows Update', 'arrows-clockwise'],
    ])
    .slice(0, 8)
  const toggles = r
    .shuffle([
      'Night light',
      'HDR',
      'Notifications',
      'Battery saver',
      'Auto-rotate',
      'Sound enhancements',
    ])
    .slice(0, 4)
  const navHtml = nav
    .map(([name, ic], i) =>
      target(
        name,
        'sidebar item',
        'div',
        `style="display:flex;gap:12px;align-items:center;padding:9px 14px;margin:2px 8px;border-radius:6px;${i === 0 ? `background:${t.hover}` : ''}"`,
        `${icon(ic, 18)}<span>${esc(name)}</span>`,
      ),
    )
    .join('')
  const rows = toggles
    .map(
      (
        name,
        i,
      ) => `<div class="card" style="display:flex;justify-content:space-between;align-items:center;padding:14px 18px;margin-bottom:8px;width:${r.int(640, 760)}px">
        <span>${esc(name)}</span>
        ${target(name, 'toggle switch', 'button', `role="switch" aria-label="${name}" style="width:44px;height:22px;border-radius:11px;border:1px solid ${t.muted};background:${i % 2 ? accent : 'transparent'}"`, '')}
      </div>`,
    )
    .join('')
  return `${titleBar(t, 'Settings', 1920)}
    <div class="abs" style="left:0;top:33px;width:300px;height:1047px;background:${t.bg}">${navHtml}</div>
    <div class="abs" style="left:340px;top:70px">
      ${target('Find a setting', 'search box', 'input', `type="text" placeholder="Find a setting" style="width:${r.int(320, 420)}px"`, '').replace('></input>', '>')}
      <h1 style="font-size:${f + 14}px;font-weight:600;margin:28px 0 18px">${esc(nav[0][0])}</h1>
      ${rows}
      <div style="display:flex;gap:12px;margin-top:18px;align-items:center">
        ${target('Apply', 'button', 'button', 'class="btn primary"', 'Apply')}
        ${target('Reset to default', 'button', 'button', 'class="btn"', 'Reset to default')}
        ${target('Learn more', 'link', 'a', 'href="#"', 'Learn more')}
      </div>
    </div>`
}

function editor(r, t, accent, f) {
  const menus = r.shuffle(['File', 'Edit', 'View', 'Selection', 'Go', 'Run', 'Terminal', 'Help'])
  const tools = r.shuffle([
    ['Save', 'floppy-disk'],
    ['Undo', 'arrow-counter-clockwise'],
    ['Redo', 'arrow-clockwise'],
    ['Search', 'magnifying-glass'],
    ['Settings', 'gear'],
    ['Copy', 'copy'],
    ['Print', 'printer'],
    ['Delete', 'trash'],
  ])
  const files = r.shuffle([
    'main.rs',
    'lib.rs',
    'README.md',
    'Cargo.toml',
    'config.json',
    'index.html',
    'styles.css',
  ])
  const status = r.shuffle([
    ['Ln 12, Col 4', 'cursor position'],
    ['UTF-8', 'encoding'],
    ['Rust', 'language'],
    ['Spaces: 4', 'indentation'],
  ])
  const code = Array.from(
    { length: 34 },
    (_, i) =>
      `<div style="height:22px;white-space:pre" class="muted">${String(i + 1).padStart(3)}  ${esc(r.pick(['fn main() {', '    let value = compute(42);', '    println!("{}", value);', '}', '// TODO: handle errors', 'use std::fs;', 'struct Config { name: String }']))}</div>`,
  ).join('')
  return `${titleBar(t, 'Editor', 1920)}
    <div class="abs" style="left:0;top:33px;width:1920px;height:30px;display:flex;gap:6px;padding:3px 10px;background:${t.surface};border-bottom:1px solid ${t.line}">
      ${menus.map((m, i) => (i < 4 ? target(m, 'menu item', 'span', 'style="padding:3px 10px"', esc(m)) : `<span style="padding:3px 10px">${esc(m)}</span>`)).join('')}</div>
    <div class="abs" style="left:0;top:64px;width:1920px;height:44px;display:flex;gap:4px;padding:6px 10px;background:${t.surface};border-bottom:1px solid ${t.line}">
      ${tools.map(([n, ic], i) => (i < 5 ? target(n, 'icon button', 'button', `class="icon" aria-label="${n}" style="width:32px;height:32px"`, icon(ic, 18)) : `<button class="icon" style="width:32px;height:32px">${icon(ic, 18)}</button>`)).join('')}</div>
    <div class="abs" style="left:0;top:109px;width:1920px;height:34px;display:flex;background:${t.bg}">
      ${files
        .slice(0, 5)
        .map(
          (n, i) =>
            `${i < 2 ? target(n, 'tab', 'div', `style="padding:7px 18px;border-right:1px solid ${t.line};background:${i === 0 ? t.surface : 'transparent'}"`, esc(n)) : `<div style="padding:7px 18px;border-right:1px solid ${t.line}">${esc(n)}</div>`}`,
        )
        .join('')}</div>
    <div class="abs" style="left:0;top:143px;width:1920px;height:905px;padding:12px 24px;background:${t.surface};font-family:Consolas,monospace">${code}</div>
    <div class="abs" style="left:0;top:1048px;width:1920px;height:32px;display:flex;gap:18px;padding:5px 14px;background:${accent};color:#fff;font-size:${f - 1}px">
      ${status.map(([n], i) => (i < 2 ? target(n, 'status bar item', 'span', '', esc(n)) : `<span>${esc(n)}</span>`)).join('')}</div>`
}

function dialog(r, t, accent, f) {
  const titles = [
    'Unsaved changes',
    'Delete this file?',
    'Replace existing file?',
    'Install update',
  ]
  const title = r.pick(titles)
  const dw = r.int(520, 640)
  const dx = r.int(500, 1920 - dw - 300)
  const dy = r.int(260, 520)
  const fake = Array.from(
    { length: 8 },
    () =>
      `<div class="card" style="width:${r.int(260, 520)}px;height:${r.int(60, 120)}px;margin:14px 0"></div>`,
  ).join('')
  return `${titleBar(t, 'Documents', 1920)}
    <div class="abs" style="left:60px;top:70px;opacity:.6">${fake}</div>
    <div class="abs" style="left:0;top:0;width:1920px;height:1080px;background:rgba(0,0,0,.45)"></div>
    <div class="abs card" style="left:${dx}px;top:${dy}px;width:${dw}px;padding:20px 24px;box-shadow:0 12px 40px rgba(0,0,0,.4)">
      <div style="display:flex;justify-content:space-between;align-items:center">
        <b style="font-size:${f + 3}px">${esc(title)}</b>
        <span style="display:flex;gap:4px">
          ${target('Help', 'icon button', 'button', 'class="icon" aria-label="Help" style="width:28px;height:28px"', icon('question', 16))}
          ${target('Close dialog', 'icon button', 'button', 'class="icon" aria-label="Close dialog" style="width:28px;height:28px"', icon('x', 16))}
        </span></div>
      <p class="muted" style="margin:16px 0">Your changes will be lost if you continue. This action cannot be undone.</p>
      <label style="display:flex;gap:8px;align-items:center;margin:10px 0 24px">
        ${target("Don't ask me again", 'checkbox', 'input', 'type="checkbox"', '').replace('></input>', '>')}<span>Don't ask me again</span></label>
      <div style="display:flex;gap:10px;justify-content:flex-end">
        ${target('Cancel', 'button', 'button', 'class="btn"', 'Cancel')}
        ${target("Don't save", 'button', 'button', 'class="btn"', "Don't save")}
        ${target('Save', 'button', 'button', 'class="btn primary"', 'Save')}
      </div></div>`
}

function webpage(r, t, accent, f) {
  const nav = r.shuffle(['Home', 'Products', 'Pricing', 'Support', 'About', 'Blog']).slice(0, 5)
  const cards = Array.from(
    { length: 3 },
    () =>
      `<div class="card" style="width:340px;padding:18px"><div style="height:140px;background:${t.hover};border-radius:6px;margin-bottom:12px"></div><b>${esc(r.pick(['Keyboard Pro', 'Wireless Mouse', 'USB-C Dock', 'Studio Monitor', 'Webcam 4K']))}</b><div class="muted" style="margin:6px 0 12px">$${r.int(20, 300)}.00</div><button class="btn">Add to cart</button></div>`,
  ).join('')
  const heroBtns = r.shuffle([
    ['Get started', 'btn primary'],
    ['Learn more', 'btn'],
  ])
  return `<div class="abs" style="left:0;top:0;width:1920px;height:64px;background:${t.surface};border-bottom:1px solid ${t.line};display:flex;align-items:center;padding:0 48px;gap:30px">
      <b style="font-size:${f + 6}px;color:${accent}">Acme</b>
      ${nav.map((n, i) => (i < 3 ? target(n, 'nav link', 'a', 'href="#"', esc(n)) : `<a href="#">${esc(n)}</a>`)).join('')}
      <span style="flex:1"></span>
      ${target('Search products', 'search box', 'input', 'type="text" placeholder="Search products" style="width:260px"', '').replace('></input>', '>')}
      ${target('Cart', 'icon button', 'button', 'class="icon" aria-label="Cart" style="width:36px;height:36px"', icon('shopping-cart', 20))}
    </div>
    <div class="abs" style="left:48px;top:120px;width:1000px">
      <h1 style="font-size:${f + 30}px;margin:0 0 14px">Everything for your desk</h1>
      <p class="muted" style="font-size:${f + 3}px;margin:0 0 26px">Hand-picked gear, shipped fast, easy returns.</p>
      <div style="display:flex;gap:14px">${heroBtns.map(([n, c]) => target(n, 'button', 'button', `class="${c}" style="padding:11px 26px"`, n)).join('')}</div>
    </div>
    <div class="abs" style="left:48px;top:420px;display:flex;gap:26px">${cards}</div>
    <div class="abs card" style="right:40px;bottom:40px;width:420px;padding:18px;box-shadow:0 8px 28px rgba(0,0,0,.3)">
      <b>We use cookies</b><p class="muted" style="margin:8px 0 14px">We use cookies to improve your experience.</p>
      <div style="display:flex;gap:10px;justify-content:flex-end">
        ${target('Reject all', 'button', 'button', 'class="btn"', 'Reject all')}
        ${target('Accept all', 'button', 'button', 'class="btn primary"', 'Accept all')}</div></div>`
}

function files(r, t) {
  const names = r.shuffle([
    'report-final.pdf',
    'budget.xlsx',
    'notes.txt',
    'holiday.jpg',
    'resume.docx',
    'slides.pptx',
    'archive.zip',
    'song.mp3',
    'invoice-0412.pdf',
    'logo.svg',
  ])
  const tree = r.shuffle([
    'Desktop',
    'Downloads',
    'Documents',
    'Pictures',
    'Music',
    'Videos',
    'OneDrive',
  ])
  const cmds = r.shuffle([
    ['New', 'plus'],
    ['Cut', 'scissors'],
    ['Copy', 'copy'],
    ['Paste', 'clipboard'],
    ['Rename', 'pencil-simple'],
    ['Share', 'share-network'],
    ['Delete', 'trash'],
  ])
  const rows = names
    .map(
      (n, i) =>
        `${i < 3 ? target(n, 'file', 'div', `style="display:flex;gap:14px;align-items:center;padding:8px 14px;border-bottom:1px solid ${t.line}"`, `${icon('file', 18)}<span style="width:340px">${esc(n)}</span><span class="muted">${r.int(12, 990)} KB</span>`) : `<div style="display:flex;gap:14px;align-items:center;padding:8px 14px;border-bottom:1px solid ${t.line}">${icon('file', 18)}<span style="width:340px">${esc(n)}</span><span class="muted">${r.int(12, 990)} KB</span></div>`}`,
    )
    .join('')
  return `${titleBar(t, 'File Explorer', 1920)}
    <div class="abs" style="left:0;top:33px;width:1920px;height:48px;display:flex;gap:6px;align-items:center;padding:0 14px;background:${t.surface};border-bottom:1px solid ${t.line}">
      ${cmds.map(([n, ic], i) => (i < 3 ? target(n, 'toolbar button', 'button', 'class="icon" style="padding:6px 12px;gap:8px"', `${icon(ic, 16)}<span>${esc(n)}</span>`) : `<button class="icon" style="padding:6px 12px;gap:8px">${icon(ic, 16)}<span>${esc(n)}</span></button>`)).join('')}
      <span style="flex:1"></span>
      ${target('Search files', 'search box', 'input', 'type="text" placeholder="Search files" style="width:260px"', '').replace('></input>', '>')}</div>
    <div class="abs" style="left:0;top:82px;width:280px;height:998px;background:${t.bg};padding-top:10px">
      ${tree.map((n, i) => (i < 3 ? target(n, 'sidebar item', 'div', 'style="padding:8px 18px;display:flex;gap:10px;align-items:center"', `${icon('folder-simple', 18)}<span>${esc(n)}</span>`) : `<div style="padding:8px 18px;display:flex;gap:10px;align-items:center">${icon('folder-simple', 18)}<span>${esc(n)}</span></div>`)).join('')}</div>
    <div class="abs" style="left:300px;top:100px;width:1000px">${rows}</div>`
}

const BUILDERS = { settings, editor, dialog, webpage, files }

/** The scene list: each layout in a light and a dark variant. */
export const SCENES = Object.keys(BUILDERS).flatMap((kind, i) => [
  { id: `${kind}-light`, kind, seed: 1000 + i * 10 + 1, theme: 'light' },
  { id: `${kind}-dark`, kind, seed: 1000 + i * 10 + 2, theme: 'dark' },
])

const ACCENTS = ['#0067c0', '#c42b1c', '#107c10', '#8764b8', '#da3b01']

export function renderScene(scene) {
  const r = rng(scene.seed)
  const theme = THEMES[scene.theme]
  const accent = r.pick(ACCENTS)
  const fontPx = r.pick([13, 14, 15])
  const body = BUILDERS[scene.kind](r, theme, accent, fontPx)
  return `<!doctype html><html><head><meta charset="utf-8"><style>${baseCss(theme, accent, fontPx)}</style></head>
<body>${body}<pre id="truth" style="display:none"></pre>
<script>
const targets = [...document.querySelectorAll('[data-target]')].map(e => { const b = e.getBoundingClientRect(); return { name: e.dataset.name, role: e.dataset.role, l: b.left, t: b.top, w: b.width, h: b.height } });
document.getElementById('truth').textContent = JSON.stringify({ scene: ${JSON.stringify(scene.id)}, w: 1920, h: 1080, targets });
</script></body></html>`
}