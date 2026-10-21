import type { FlightCall, FlightEvent, FlightHeader, FlightStep } from '@/config/flight'

export const START = '2026-10-20T10:00:00Z'

export function makeStep(n: number, seconds: number, extra: Partial<FlightStep> = {}): FlightStep {
  const at = new Date(Date.parse(START) + seconds * 1000).toISOString().replace('.000Z', 'Z')
  return {
    step: n,
    startedUtc: at,
    goal: null,
    tool: 'click',
    argsJson: '{"x":500,"y":400}',
    reason: `Reason ${n}`,
    riskStated: null,
    riskFinal: null,
    riskReasonsJson: null,
    gate: null,
    result: 'ok',
    error: null,
    observationPath: null,
    observationFile: null,
    promptText: null,
    thinkingChars: null,
    completionTokens: null,
    phases: {
      observe: 100,
      screenCheck: null,
      finishCheck: null,
      decide: 800,
      gate: null,
      act: 50,
      settle: 200,
    },
    ...extra,
  }
}

export function makeHeader(extra: Partial<FlightHeader> = {}): FlightHeader {
  return {
    id: 'r1',
    task: 'Open notepad',
    source: 'built-in',
    model: 'm',
    startedUtc: START,
    endedUtc: '2026-10-20T10:00:30Z',
    outcome: 'succeeded',
    summary: null,
    plan: null,
    steps: 3,
    retries: 0,
    durationSecs: 30,
    verdict: null,
    video: null,
    sessionDir: null,
    ...extra,
  }
}

export function makeCall(id: number, step: number, extra: Partial<FlightCall> = {}): FlightCall {
  return {
    id,
    step,
    kind: 'decide',
    attempt: 1,
    startedUtc: START,
    durationMs: 100,
    waitBeforeMs: 0,
    status: 200,
    outcome: 'ok',
    error: null,
    truncated: false,
    model: 'm',
    promptTokens: null,
    completionTokens: null,
    ...extra,
  }
}

export const noEvents: FlightEvent[] = []

export function stubResizeObserver(width = 800, height = 400) {
  class Fake {
    callback: ResizeObserverCallback
    constructor(callback: ResizeObserverCallback) {
      this.callback = callback
    }
    observe(target: Element) {
      this.callback(
        [
          {
            target,
            contentRect: { width, height } as DOMRectReadOnly,
            contentBoxSize: [{ inlineSize: width, blockSize: height }],
            borderBoxSize: [{ inlineSize: width, blockSize: height }],
            devicePixelContentBoxSize: [],
          } as unknown as ResizeObserverEntry,
        ],
        this as unknown as ResizeObserver,
      )
    }
    unobserve() {}
    disconnect() {}
  }
  globalThis.ResizeObserver = Fake as unknown as typeof ResizeObserver
  window.ResizeObserver = Fake as unknown as typeof ResizeObserver
  Object.defineProperty(HTMLElement.prototype, 'offsetWidth', {
    configurable: true,
    get: () => width,
  })
  Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
    configurable: true,
    get: () => height,
  })
}