const START = Date.parse('2026-10-20T10:00:00Z')
const at = (seconds: number) => new Date(START + seconds * 1000).toISOString().replace('.000Z', 'Z')

const none = {
  observe: null,
  screenCheck: null,
  finishCheck: null,
  decide: null,
  gate: null,
  act: null,
  settle: null,
}

function step(n: number, seconds: number, extra: Record<string, unknown>) {
  return {
    step: n,
    startedUtc: at(seconds),
    goal: 1,
    tool: 'click',
    argsJson: null,
    reason: null,
    riskStated: 'low',
    riskFinal: 'low',
    riskReasonsJson: '[]',
    gate: 'allowed',
    result: 'ok',
    error: null,
    observationPath: `C:/fixture/${n}.jpg`,
    observationFile: `screenshots/${n}.jpg`,
    promptText: `Task: open notepad and type a note.\nControls:\ne1 button Start\ne2 text Search`,
    thinkingChars: 480,
    completionTokens: 96,
    phases: { ...none, observe: 320, decide: 2100, gate: 4, act: 180, settle: 400 },
    ...extra,
  }
}

export const flightSteps = [
  step(1, 0, {
    tool: 'press_keys',
    argsJson: '{"keys":["win"]}',
    reason: 'Open the Start menu to find Notepad.',
  }),
  step(2, 6, {
    tool: 'type_text',
    argsJson: '{"text":"notepad"}',
    reason: 'Type the name of the app in the search box.',
  }),
  step(3, 14, {
    tool: 'press_keys',
    argsJson: '{"keys":["enter"]}',
    reason: 'Launch the first result.',
  }),
  step(4, 23, {
    tool: 'click',
    argsJson: '{"x":520,"y":410}',
    reason: 'Focus the empty document.',
    result: 'failed',
    error: 'the window was not in front',
  }),
  step(5, 31, {
    tool: 'click',
    argsJson: '{"x":480,"y":380}',
    reason: 'Click into the text area of Notepad.',
  }),
  step(6, 40, {
    tool: 'type_text',
    argsJson: '{"text":"Buy milk and call Sam"}',
    reason: 'Write the note the user asked for.',
  }),
]

export const flightRuns = [
  {
    id: '110000-1-insight',
    task: 'What is on this screen?',
    source: 'screen-insight',
    outcome: 'succeeded',
    startedUtc: at(0),
    endedUtc: at(5),
    steps: 0,
    verdict: null,
    thumb: null,
  },
  {
    id: '100000-1-open-notepad',
    task: 'Open Notepad and write a note: buy milk and call Sam',
    source: 'built-in',
    outcome: 'succeeded',
    startedUtc: at(0),
    endedUtc: at(55),
    steps: 6,
    verdict: 'verified',
    thumb: 'screenshots/3.jpg',
  },
  {
    id: '090000-1-old',
    task: 'An older session from before the flight recorder had video',
    source: 'Claude Code',
    outcome: 'failed',
    startedUtc: '2026-10-19T09:00:00Z',
    endedUtc: '2026-10-19T09:00:30Z',
    steps: 3,
    verdict: null,
    thumb: null,
  },
]

export function flightHeader(id: string, withVideo: boolean) {
  const run = flightRuns.find((item) => item.id === id) ?? flightRuns[0]!
  return {
    id: run.id,
    task: run.task,
    source: run.source,
    model: 'gemma-4-12b-it',
    startedUtc: run.startedUtc,
    endedUtc: run.endedUtc,
    outcome: run.outcome,
    summary: 'Wrote the note.',
    plan: ['Open Notepad', 'Write the note'],
    steps: run.steps,
    retries: 1,
    durationSecs: 55,
    verdict: {
      result: 'verified',
      evidence: 'The note is on screen in Notepad.',
      mode: 'background',
      error: null,
    },
    video: withVideo ? { file: 'session.mp4', kind: 'storyboard', frames: 6, seconds: 55 } : null,
    sessionDir: 'C:/fixture',
    analysis:
      run.source === 'screen-insight'
        ? {
            answer:
              'A settings dialog with a name field and a red Delete button. The user can fill in the name or confirm.',
            findings: [
              {
                n: 1,
                label: 'Name field',
                kind: 'control',
                box: [156, 250, 563, 306],
                description: 'A text field for a name.',
                evidence: 'light grey input with an empty value',
              },
              {
                n: 2,
                label: 'Delete',
                kind: 'issue',
                box: [703, 778, 844, 845],
                description: 'A red button that probably deletes the entry.',
                evidence: 'red fill, bottom right',
              },
            ],
            model: 'gemma-4-12b-it',
            capturedUtc: run.startedUtc,
            captureFile: 'screenshots/3.jpg',
            error: null,
          }
        : null,
  }
}

export const flightEvents = [
  { id: 1, atUtc: at(20), kind: 'stuck_warning', dataJson: '{}' },
  { id: 2, atUtc: at(36), kind: 'ask_user', dataJson: '{"question":"Save the note?"}' },
]

export const flightCalls = [
  call(1, 2, 1, 'retried', 503),
  call(2, 2, 2, 'ok', 200),
  call(3, 3, 1, 'ok', 200),
]

function call(id: number, stepNo: number, attempt: number, outcome: string, status: number) {
  return {
    id,
    step: stepNo,
    kind: 'decide',
    attempt,
    startedUtc: at(7),
    durationMs: attempt === 1 ? 900 : 2100,
    waitBeforeMs: attempt === 1 ? 0 : 500,
    status,
    outcome,
    error: outcome === 'retried' ? 'HTTP 503' : null,
    truncated: false,
    model: 'gemma-4-12b-it',
    promptTokens: 1800,
    completionTokens: 96,
  }
}

export function drawnShot(n: number): string {
  const canvas = document.createElement('canvas')
  canvas.width = 960
  canvas.height = 540
  const ctx = canvas.getContext('2d')
  if (!ctx) return ''
  ctx.fillStyle = '#2b3a4a'
  ctx.fillRect(0, 0, 960, 540)
  ctx.fillStyle = '#f2f2f0'
  ctx.fillRect(160, 80, 640, 360)
  ctx.fillStyle = '#d9d9d6'
  ctx.fillRect(160, 80, 640, 28)
  ctx.fillStyle = '#222'
  ctx.font = '28px sans-serif'
  ctx.fillText(`Step ${n}`, 190, 200)
  return canvas.toDataURL('image/png')
}