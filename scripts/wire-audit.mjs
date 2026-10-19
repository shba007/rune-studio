// Captures what a live rune-infer really accepts and returns, so the wire format Studio relies on
// can be checked against the server instead of against docs.
//
//   RUNE_BASE_URL=http://127.0.0.1:8080 RUNE_API_KEY=sk-... node scripts/wire-audit.mjs
//
// Environment: RUNE_BASE_URL (default http://127.0.0.1:8080), RUNE_API_KEY, RUNE_MODEL (the omni
// model, default gemma-4-12b-it), RUNE_DECISION_MODEL (default liquid-d1-3b).
// Writes docs/wire-audit-<YYYY-MM-DD>.md and prints the same table. Exits 1 when no server answers.
// No dependencies; the API key is only ever sent as a header and is scrubbed from all output.

import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { deflateSync } from 'node:zlib'

const BASE = (process.env.RUNE_BASE_URL || 'http://127.0.0.1:8080').replace(/\/+$/, '')
const KEY = process.env.RUNE_API_KEY || ''
const MODEL = process.env.RUNE_MODEL || 'gemma-4-12b-it'
const DECISION_MODEL = process.env.RUNE_DECISION_MODEL || 'liquid-d1-3b'
const TIMEOUT_MS = 120_000
const RAW_LIMIT = 1500

// ---------------------------------------------------------------- images and sound, no libraries

const CRC_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    table[n] = c >>> 0
  }
  return table
})()

function crc32(bytes) {
  let c = 0xffffffff
  for (const byte of bytes) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}

function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length)
  out.writeUInt32BE(data.length, 0)
  out.write(type, 4, 'ascii')
  data.copy(out, 8)
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length)
  return out
}

/** An 8-bit RGB PNG. `pixel(x, y)` returns [r, g, b]. */
function png(width, height, pixel) {
  const raw = Buffer.alloc((width * 3 + 1) * height)
  for (let y = 0; y < height; y++) {
    const row = y * (width * 3 + 1)
    raw[row] = 0 // filter: none
    for (let x = 0; x < width; x++) {
      const [r, g, b] = pixel(x, y)
      raw[row + 1 + x * 3] = r
      raw[row + 2 + x * 3] = g
      raw[row + 3 + x * 3] = b
    }
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(width, 0)
  header.writeUInt32BE(height, 4)
  header[8] = 8 // bit depth
  header[9] = 2 // colour type: RGB
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

/** 16 kHz mono 16-bit WAV: one second of a quiet 440 Hz tone. */
function wav() {
  const rate = 16000
  const samples = rate
  const data = Buffer.alloc(samples * 2)
  for (let i = 0; i < samples; i++)
    data.writeInt16LE(Math.round(Math.sin((2 * Math.PI * 440 * i) / rate) * 3000), i * 2)
  const header = Buffer.alloc(44)
  header.write('RIFF', 0, 'ascii')
  header.writeUInt32LE(36 + data.length, 4)
  header.write('WAVEfmt ', 8, 'ascii')
  header.writeUInt32LE(16, 16)
  header.writeUInt16LE(1, 20) // PCM
  header.writeUInt16LE(1, 22) // mono
  header.writeUInt32LE(rate, 24)
  header.writeUInt32LE(rate * 2, 28)
  header.writeUInt16LE(2, 32)
  header.writeUInt16LE(16, 34)
  header.write('data', 36, 'ascii')
  header.writeUInt32LE(data.length, 40)
  return Buffer.concat([header, data])
}

const dataUri = (mime, bytes) => `data:${mime};base64,${bytes.toString('base64')}`

const RED = png(128, 128, () => [220, 20, 20])
// White 320x240 picture with one blue 60x60 square, left edge x=200, top edge y=60.
const SQUARE = { x: 200, y: 60, size: 60, width: 320, height: 240 }
const SQUARE_PNG = png(SQUARE.width, SQUARE.height, (x, y) =>
  x >= SQUARE.x && x < SQUARE.x + SQUARE.size && y >= SQUARE.y && y < SQUARE.y + SQUARE.size
    ? [20, 60, 220]
    : [255, 255, 255],
)
const WAV = wav()

// ---------------------------------------------------------------- requests and the report

const rows = []

/** What goes into the report: no base64, no key, cut to RAW_LIMIT characters. */
function scrub(text) {
  let out = String(text)
    .replace(/data:[a-z]+\/[a-z0-9.+-]+;base64,[A-Za-z0-9+/=]+/gi, '<base64 removed>')
    .replace(/[A-Za-z0-9+/=]{200,}/g, '<base64 removed>')
  if (KEY) out = out.split(KEY).join('<key>')
  return out.length > RAW_LIMIT ? `${out.slice(0, RAW_LIMIT)}… (${out.length} characters)` : out
}

async function send(method, path, body) {
  const headers = { Accept: 'application/json' }
  if (body !== undefined) headers['Content-Type'] = 'application/json'
  if (KEY) headers.Authorization = `Bearer ${KEY}`
  const signal = AbortSignal.timeout(TIMEOUT_MS)
  const response =
    body === undefined
      ? await fetch(`${BASE}${path}`, { method: 'GET', headers, signal })
      : await fetch(`${BASE}${path}`, {
          method: 'POST',
          headers,
          body: JSON.stringify(body),
          signal,
        })
  const text = await response.text()
  let json
  try {
    json = JSON.parse(text)
  } catch {}
  return { status: response.status, ok: response.ok, text, json }
}

/** Runs one request, records a row, never throws once the server has been reached. */
async function record(name, sent, method, path, body, note) {
  let result
  try {
    result = await send(method, path, body)
  } catch (error) {
    result = {
      status: 'no answer',
      ok: false,
      text: String(error?.message || error),
      json: undefined,
    }
  }
  const extra = note ? note(result) : ''
  rows.push({
    name,
    sent,
    status: result.status,
    accepted: result.ok ? 'yes' : 'no',
    raw: scrub(result.text),
    extra,
  })
  console.log(
    `${result.ok ? 'ok ' : 'FAIL'} ${String(result.status).padEnd(9)} ${name}${extra ? `  [${extra}]` : ''}`,
  )
  return result
}

const message = (result) => result.json?.choices?.[0]?.message
const thinkingSeen = (result) => {
  const m = message(result)
  const seen = Boolean(m?.reasoning_content || m?.reasoning)
  return result.ok ? `reasoning ${seen ? 'present' : 'absent'}` : ''
}
const toolSeen = (result) =>
  result.ok ? `tool_calls ${message(result)?.tool_calls?.length ? 'present' : 'absent'}` : ''

const CLICK_TOOL = [
  {
    type: 'function',
    function: {
      name: 'click',
      description: 'Click a point on the screen',
      parameters: {
        type: 'object',
        properties: { x: { type: 'integer' }, y: { type: 'integer' } },
        required: ['x', 'y'],
      },
    },
  },
]

function chat(extra, content, tools = CLICK_TOOL) {
  return {
    model: MODEL,
    temperature: 0,
    max_tokens: 256,
    messages: [
      { role: 'system', content: 'You control a computer. Call a tool.' },
      { role: 'user', content },
    ],
    ...(tools ? { tools } : {}),
    ...extra,
  }
}

async function main() {
  // Reachability first: with no server there is nothing to audit.
  try {
    await send('GET', '/health')
  } catch (error) {
    console.error(
      `No server answered at ${BASE} (${error?.message || error}). Set RUNE_BASE_URL and start rune-infer.`,
    )
    process.exit(1)
  }

  // 1. Discovery.
  await record('health', 'GET /health', 'GET', '/health')
  await record('readyz', 'GET /readyz', 'GET', '/readyz')
  await record('models', 'GET /v1/models', 'GET', '/v1/models')
  await record('model', `GET /v1/models/${MODEL}`, 'GET', `/v1/models/${encodeURIComponent(MODEL)}`)

  // 2. Chat with an image and one tool.
  const screenshot = [
    { type: 'image_url', image_url: { url: dataUri('image/png', RED) } },
    { type: 'text', text: 'Click the middle of the picture.' },
  ]
  for (const choice of ['required', 'auto']) {
    await record(
      `chat + PNG + tool, tool_choice=${choice}`,
      `image_url part (PNG data URI), tools[1], tool_choice: "${choice}"`,
      'POST',
      '/v1/chat/completions',
      chat({ tool_choice: choice }, screenshot),
      toolSeen,
    )
  }

  // 3. The thinking switch: three spellings, off and on, same prompt, no tools so the text is visible.
  const prompt = 'What is 17 times 23? Answer with the number only.'
  const switches = [
    {
      name: 'chat_template_kwargs.enable_thinking',
      make: (on) => ({ chat_template_kwargs: { enable_thinking: on } }),
      off: 'false',
      on: 'true',
    },
    {
      name: 'thinking',
      make: (on) => ({ thinking: { type: on ? 'enabled' : 'disabled' } }),
      off: '{"type":"disabled"}',
      on: '{"type":"enabled"}',
    },
    {
      name: 'reasoning_effort',
      make: (on) => ({ reasoning_effort: on ? 'high' : 'none' }),
      off: '"none"',
      on: '"high"',
    },
  ]
  for (const { name, make, off, on } of switches) {
    await record(
      `thinking off: ${name}`,
      `${name}: ${off}`,
      'POST',
      '/v1/chat/completions',
      chat(make(false), prompt, null),
      thinkingSeen,
    )
    await record(
      `thinking on: ${name}`,
      `${name}: ${on}`,
      'POST',
      '/v1/chat/completions',
      chat(make(true), prompt, null),
      thinkingSeen,
    )
  }

  // 4. Audio, two spellings of the same one-second clip.
  const audioText = { type: 'text', text: 'What do you hear? Answer in a few words.' }
  await record(
    'audio: audio_url (Studio)',
    'audio_url part {url: data:audio/wav;base64,...}',
    'POST',
    '/v1/chat/completions',
    chat(
      {},
      [{ type: 'audio_url', audio_url: { url: dataUri('audio/wav', WAV) } }, audioText],
      null,
    ),
  )
  await record(
    'audio: input_audio (spec)',
    'input_audio part {data, format: "wav"}',
    'POST',
    '/v1/chat/completions',
    chat(
      {},
      [
        { type: 'input_audio', input_audio: { data: WAV.toString('base64'), format: 'wav' } },
        audioText,
      ],
      null,
    ),
  )

  // 5. Decisions, with and without the picture. The raw answer shows which shape the server uses.
  const question = { name: 'red', type: 'noul', instruction: 'Is this image mostly red?' }
  await record(
    'decisions with image',
    'state {text, images:[PNG data URI]}, question type "noul"',
    'POST',
    '/v1/decisions',
    {
      model: DECISION_MODEL,
      state: { text: 'Is this image mostly red?', images: [dataUri('image/png', RED)] },
      questions: [question],
    },
    (r) =>
      r.json?.answers
        ? `answer: ${scrub(JSON.stringify(r.json.answers.red ?? r.json.answers))}`
        : '',
  )
  await record(
    'decisions without image',
    'state: "Is this image mostly red?" (text only), question type "noul"',
    'POST',
    '/v1/decisions',
    { model: DECISION_MODEL, state: 'Is this image mostly red?', questions: [question] },
    (r) =>
      r.json?.answers
        ? `answer: ${scrub(JSON.stringify(r.json.answers.red ?? r.json.answers))}`
        : '',
  )

  // 6. Grounding: one blue square on white. Read the fields against the known square to learn units and axis order.
  const [left, top, size] = [SQUARE.x, SQUARE.y, SQUARE.size].map(String)
  const [cx, cy] = [SQUARE.x + SQUARE.size / 2, SQUARE.y + SQUARE.size / 2]
  const [nx, ny] = [
    Math.round((cx / SQUARE.width) * 1000),
    Math.round((cy / SQUARE.height) * 1000),
  ].map(String)
  const where = `image (PNG ${String(SQUARE.width)}x${String(SQUARE.height)}, ${size}x${size} square with its top-left at (${left}, ${top}), centre (${String(cx)}, ${String(cy)}) px = (${nx}, ${ny}) in 0..1000), query "square"`
  await record(
    'grounding',
    where,
    'POST',
    '/v1/images/grounding',
    { model: MODEL, image: dataUri('image/png', SQUARE_PNG), query: 'square' },
    (r) =>
      ['point', 'point_pixels', 'box_2d', 'box_normalized']
        .map((field) => `${field}=${JSON.stringify(r.json?.[field])}`)
        .join(' '),
  )

  report()
}

const cell = (text) => String(text).replace(/\|/g, '\\|').replace(/\r?\n/g, ' ')

function report() {
  const today = new Date().toISOString().slice(0, 10)
  const lines = [
    `# Wire audit ${today}`,
    '',
    `Server: \`${BASE}\`. Omni model: \`${MODEL}\`. Decision model: \`${DECISION_MODEL}\`. API key: ${KEY ? 'sent' : 'not set'}.`,
    'Generated by `scripts/wire-audit.mjs`. Raw responses are cut to 1,500 characters with base64 removed.',
    '',
    '| Request | What was sent | HTTP | Accepted | Notes | Raw response |',
    '| --- | --- | --- | --- | --- | --- |',
    ...rows.map(
      (row) =>
        `| ${cell(row.name)} | ${cell(row.sent)} | ${row.status} | ${row.accepted} | ${cell(row.extra)} | \`${cell(row.raw).replace(/`/g, "'")}\` |`,
    ),
    '',
  ]
  const here = dirname(fileURLToPath(import.meta.url))
  const docs = join(here, '..', 'docs')
  mkdirSync(docs, { recursive: true })
  const file = join(docs, `wire-audit-${today}.md`)
  writeFileSync(file, lines.join('\n'))
  console.log(`\nWrote ${file}`)
}

void main()