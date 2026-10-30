// Draws the Rune Studio logo: the dot-matrix orb as a mark, dark dots on a white disc.
//
//   node scripts/make-logo.mjs
//
// Writes public/logo.svg (full colour, the source for every app icon) and
// src/assets/icons/rune-mark.svg (single colour, for the UI). Then run
// `bun run tauri icon public/logo.svg` to make the icon set.

import { mkdirSync, writeFileSync } from 'node:fs'

const GRID = 9
const CENTER = (GRID - 1) / 2
const REACH = 4.35 // dots inside this radius (in grid steps) form the disc
const RING = [3.2, 4.4]
const HEAD = (-50 * Math.PI) / 180 // the bright head of the scan, up and to the right
const TAIL = Math.PI * 1.15

/** Every dot of the mark with its brightness (0..1) and size (0..1). */
function dots() {
  const out = []
  for (let gx = 0; gx < GRID; gx++) {
    for (let gy = 0; gy < GRID; gy++) {
      const dx = gx - CENTER
      const dy = gy - CENTER
      const r = Math.hypot(dx, dy)
      if (r > REACH) continue
      let light = 0.2
      let size = 0.55
      if (r < 0.5) {
        light = 1
        size = 1
      } else if (r >= RING[0] && r <= RING[1]) {
        const behind = (((HEAD - Math.atan2(dy, dx)) % (Math.PI * 2)) + Math.PI * 4) % (Math.PI * 2)
        const glow = Math.max(0, 1 - behind / TAIL)
        light = 0.45 + 0.55 * glow
        size = 0.7 + 0.3 * glow
      }
      out.push({ gx, gy, light, size })
    }
  }
  return out
}

const round = (value) => Math.round(value * 100) / 100

function colour() {
  const step = 80
  const circles = dots()
    .map(({ gx, gy, light, size }) => {
      const x = 512 + (gx - CENTER) * step
      const y = 512 + (gy - CENTER) * step
      return `    <circle cx="${x}" cy="${y}" r="${round(20 + 16 * size)}" fill-opacity="${round(light)}"/>`
    })
    .join('\n')
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <title>Rune Studio</title>
  <circle cx="512" cy="512" r="496" fill="#f1f1ef"/>
  <circle cx="512" cy="512" r="496" fill="none" stroke="#141414" stroke-opacity="0.35" stroke-width="16"/>
  <g fill="#141414">
${circles}
  </g>
</svg>
`
}

function mono() {
  const step = 2.4
  const circles = dots()
    .map(({ gx, gy, light, size }) => {
      const x = round(12 + (gx - CENTER) * step)
      const y = round(12 + (gy - CENTER) * step)
      return `    <circle cx="${x}" cy="${y}" r="${round(0.6 + 0.5 * size)}" fill-opacity="${round(light)}"/>`
    })
    .join('\n')
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
  <g fill="currentColor">
${circles}
  </g>
</svg>
`
}

mkdirSync('public', { recursive: true })
writeFileSync('public/logo.svg', colour())
writeFileSync('src/assets/icons/rune-mark.svg', mono())
console.log('wrote public/logo.svg and src/assets/icons/rune-mark.svg')