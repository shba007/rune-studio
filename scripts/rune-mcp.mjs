#!/usr/bin/env node
// Connects an MCP client (Claude Code) to Rune Studio and starts the app when it is not running.
//
// Claude Code talks to this script over stdio. The script forwards every message to the app's
// local MCP door (http://127.0.0.1:47600/mcp) and launches the built app first if nothing is
// listening, so nothing has to be installed or started by hand.
//
//   claude mcp add --scope user rune-studio -- node /path/to/scripts/rune-mcp.mjs
//   node scripts/rune-mcp.mjs --check        # print what it can see, then exit
//
// Needs the app built with the agent-bridge feature (see docs/AGENT_GUIDE.md), and "Allow outside
// agent" switched on once in Settings. RUNE_STUDIO_EXE overrides where the app is looked for.

import { execFile, spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import net from 'node:net'
import { dirname, join } from 'node:path'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const exe =
  process.env.RUNE_STUDIO_EXE ??
  join(here, '..', 'src-tauri', 'target', 'release', 'rune-studio.exe')
const port = Number(process.env.RUNE_BRIDGE_PORT ?? 47600)
const url = `http://127.0.0.1:${port}/mcp`
const START_TIMEOUT_MS = 30_000

const log = (text) => process.stderr.write(`[rune-mcp] ${text}\n`)
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

function portOpen() {
  return new Promise((resolve) => {
    const socket = net.connect({ host: '127.0.0.1', port })
    const done = (open) => {
      socket.destroy()
      resolve(open)
    }
    socket.setTimeout(500, () => done(false))
    socket.once('connect', () => done(true))
    socket.once('error', () => done(false))
  })
}

/** Starts the app if nothing is listening, then waits until the door answers. */
async function ensureApp() {
  if (await portOpen()) return
  if (!existsSync(exe)) {
    throw new Error(
      `Rune Studio is not built yet (looked for ${exe}). Build it with: bun run tauri build --no-bundle --features agent-bridge`,
    )
  }
  log(`starting ${exe}`)
  spawn(exe, [], { detached: true, stdio: 'ignore', cwd: dirname(exe) }).unref()
  const deadline = Date.now() + START_TIMEOUT_MS
  while (Date.now() < deadline) {
    if (await portOpen()) return
    await sleep(400)
  }
  throw new Error(
    `Rune Studio started but nothing is listening on port ${port}. Is it a build with --features agent-bridge?`,
  )
}

let cachedToken = null
/** The app prints its own token; it is never stored in a file. */
function token() {
  if (cachedToken) return Promise.resolve(cachedToken)
  return new Promise((resolve, reject) => {
    execFile(
      exe,
      ['--print-bridge-token'],
      { timeout: 10_000, windowsHide: true },
      (error, stdout, stderr) => {
        const value = stdout.trim()
        if (error || !value)
          reject(
            new Error(
              `could not read the bridge token: ${stderr.trim() || error?.message || 'empty'}`,
            ),
          )
        else resolve((cachedToken = value))
      },
    )
  })
}

/** Sends one JSON-RPC message to the app. Resolves to the reply, or null when none is expected. */
async function forward(message) {
  await ensureApp()
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${await token()}` },
    body: JSON.stringify(message),
  })
  if (response.status === 202) return null
  const text = await response.text()
  if (!response.ok) {
    let detail = text
    try {
      const body = JSON.parse(text)
      detail = `${body.error}: ${body.message}`
    } catch {
      // Keep the raw text.
    }
    throw new Error(detail)
  }
  return JSON.parse(text)
}

async function check() {
  console.log(`app file:   ${exe} (${existsSync(exe) ? 'found' : 'NOT FOUND'})`)
  console.log(
    `running:    ${(await portOpen()) ? 'yes' : 'no (it will be started on the first call)'}`,
  )
  if (existsSync(exe)) {
    try {
      console.log(`token:      ${(await token()) ? 'available' : 'missing'}`)
    } catch (error) {
      console.log(`token:      ${error.message}`)
    }
  }
}

async function serve() {
  let queue = Promise.resolve()
  const lines = createInterface({ input: process.stdin })
  lines.on('line', (line) => {
    if (!line.trim()) return
    let message
    try {
      message = JSON.parse(line)
    } catch {
      return log('ignored a line that was not JSON')
    }
    // One at a time keeps replies in the order the client sent the requests.
    queue = queue.then(async () => {
      try {
        const reply = await forward(message)
        if (reply) process.stdout.write(`${JSON.stringify(reply)}\n`)
      } catch (error) {
        log(error.message)
        if (message.id !== undefined) {
          process.stdout.write(
            `${JSON.stringify({ jsonrpc: '2.0', id: message.id, error: { code: -32000, message: error.message } })}\n`,
          )
        }
      }
    })
  })
  lines.on('close', () => queue.then(() => process.exit(0)))
}

if (process.argv.includes('--check')) await check()
else await serve()