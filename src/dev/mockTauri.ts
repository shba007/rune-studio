import { mockIPC } from '@tauri-apps/api/mocks'

import {
  drawnShot,
  flightCalls,
  flightEvents,
  flightHeader,
  flightRuns,
  flightSteps,
} from './flightFixture'

const removedGoals = new Set<string>()
const monitor = () => ({
  id: 1,
  x: 0,
  y: 0,
  width: 1920,
  height: 1080,
  scaleFactor: 1,
  primary: true,
})

function placeholderThumbnail(): string {
  const canvas = document.createElement('canvas')
  canvas.width = 320
  canvas.height = 180
  const ctx = canvas.getContext('2d')
  if (!ctx) return ''
  const gradient = ctx.createLinearGradient(0, 0, 320, 180)
  gradient.addColorStop(0, '#2a2a2a')
  gradient.addColorStop(1, '#6a6a6a')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, 320, 180)
  return canvas.toDataURL('image/jpeg', 0.7)
}

function analysisFixture() {
  return {
    status: 'done',
    id: '120000-1-analysis',
    question: 'What is on this screen, and what could the user do next?',
    answer:
      'A settings dialog with two text fields, a notes area and two buttons. The user can fill in the fields and confirm with the red Delete button or cancel.',
    findings: [
      {
        n: 1,
        label: 'Title bar',
        kind: 'region',
        box: [125, 111, 875, 178],
        description: 'The dialog is called Settings.',
        evidence: 'dark bar across the top of the window',
      },
      {
        n: 2,
        label: 'Name field',
        kind: 'control',
        box: [156, 250, 563, 306],
        description: 'A text field for a name.',
        evidence: 'light grey input with an empty value',
      },
      {
        n: 3,
        label: 'Delete',
        kind: 'issue',
        box: [703, 778, 844, 845],
        description: 'A red button that probably deletes the entry.',
        evidence: 'red fill, bottom right of the dialog',
      },
      {
        n: 4,
        label: 'Footer note',
        kind: 'text',
        box: null,
        description: 'Small print that could not be placed.',
        evidence: 'mentioned at the bottom, too small to locate',
      },
    ],
    model: 'gemma-4-12b-it',
    latencyMs: 4200,
    error: null,
    createdUtc: new Date().toISOString(),
    monitor: { id: 1, x: 0, y: 0, width: 1920, height: 1080, scaleFactor: 1, primary: true },
    stale: false,
    boxesRejected: 0,
    selected: null,
  }
}

export function installTauriMock() {
  const videoParam = new URLSearchParams(location.search).get('video')
  const withVideo = videoParam !== null
  const agentParam = new URLSearchParams(location.search).get('agent')
  mockIPC(
    (command, args) => {
      const given = (args ?? {}) as Record<string, unknown>
      switch (command) {
        case 'external_agent':
          return agentParam
            ? {
                agent: agentParam,
                id: agentParam.toLowerCase(),
                known: false,
                verified: false,
                declared: agentParam,
              }
            : null
        case 'flight_runs':
          return flightRuns
        case 'flight_run':
          return flightRuns.some((run) => run.id === given.id)
            ? flightHeader(String(given.id), withVideo)
            : null
        case 'flight_steps':
          return flightSteps
        case 'flight_events':
          return flightEvents
        case 'flight_calls':
          return flightCalls.filter((item) => given.step == null || item.step === given.step)
        case 'flight_call_body':
          return [
            '{"messages":[{"role":"user","content":"typed Buy milk and call Sam"}]}',
            '{"choices":[{"message":{"reasoning_content":"The menu is open, so type the name."}}]}',
          ]
        case 'flight_file': {
          const file = String(given.relative)
          if (file.endsWith('.mp4'))
            return videoParam === 'webm' ? '/dev/flight.webm' : '/dev/flight.mp4'
          const n = Number(/(\d+)\.jpg$/.exec(file)?.[1] ?? 0)
          return n > 0 ? drawnShot(n) : file
        }
        case 'flight_reveal':
        case 'open_flight':
          return null
        case 'get_settings':
          return {
            baseUrl: 'http://127.0.0.1:8080',
            model: 'gemma-4-12b-it',
            models: {
              agent: 'gemma-4-12b-it',
              verifier: '',
              analysis: '',
              voice: '',
              decision: 'liquid-d1-3b',
              advisor: '',
            },
            maxSteps: 30,
            toggleHotkey: 'Alt+Q',
            stopHotkey: 'Escape',
            externalAgent: false,
            externalAgentSince: 0,
            showThinking: false,
            askEveryAction: false,
            inputClassification: 'automatic',
            developerMode: new URLSearchParams(location.search).get('developer') === '1',
            fastChecks: false,
            decisionModel: 'liquid-d1-3b',
            decisionUrl: '',
            archiveKeepDays: 0,
            verifyMode: 'background',
            demoMode: false,
            recordSessionVideo: 'off',
            modelImageResolution: '1920',
            recordingResolution: 'native',
            offerReview: 'always',
            reflectionEnabled: true,
            idleMinutes: 10,
          }
        case 'bridge_info':
          return {
            available: true,
            url: 'http://127.0.0.1:47600/mcp',
            command:
              'claude mcp add --scope user rune-studio -- "C:/Rune Studio/rune-studio-cli.exe" mcp --agent claude',
          }
        case 'decision_delete': {
          removedGoals.add(String(given.id))
          return null
        }
        case 'decisions_state':
          return [
            {
              id: 'goal-fit',
              revision: 3,
              text: 'I want to be physically fit.',
              class: 'health',
              kind: 'goal',
              status: 'goal_ready',
              category: 'Health',
              question: null,
              goal: null,
              approvedGoal: {
                outcome: 'Run 5 km without stopping',
                horizon: '3 months',
                current: 'Walks 30 minutes a day, no running yet.',
                success: 'Runs 5 km in one go, three times a week.',
                resources: 'Park nearby, 4 free evenings a week.',
                constraints: 'Mild knee strain; avoid hard surfaces.',
              },
              outcomeMetric: { baseline: 0, current: 1.5, target: 5, unit: 'km', note: 'Week 3' },
              pathways: {
                assessment: 'A gradual run-walk plan is realistic in the time available.',
                similarOutcome: '',
                selected: 'route-a',
                approved: null,
                options: [
                  {
                    id: 'route-a',
                    title: 'Run-walk intervals',
                    summary: 'Alternate running and walking, lengthening the runs each week.',
                    rationale: 'Lowest injury risk for a returning runner.',
                    pros: ['Gentle on the knee'],
                    cons: ['Slower at first'],
                    duration: '10 to 12 weeks',
                    resources: ['Running shoes'],
                    feasibility: 'feasible',
                    missingInformation: [],
                  },
                  {
                    id: 'route-b',
                    title: 'Treadmill base building',
                    summary: 'Build endurance on a treadmill before moving outdoors.',
                    rationale: 'Controlled surface, but needs gym access.',
                    pros: ['Controlled pace'],
                    cons: ['Needs a gym'],
                    duration: '12 weeks',
                    resources: ['Gym membership'],
                    feasibility: 'conditional',
                    missingInformation: ['Gym access'],
                  },
                ],
              },
              error: null,
              taskId: null,
            },
            {
              id: 'goal-save',
              revision: 1,
              text: 'Save money for a trip',
              class: 'finance',
              kind: 'goal',
              status: 'clarifying',
              category: 'Finance',
              question: {
                id: 'q1',
                revision: 1,
                kind: 'clarification',
                title: 'When do you want to travel?',
                explanation: 'The timeline decides how much to save each month.',
                options: [
                  { id: 'o1', label: 'In 6 months', explanation: 'A short, focused saving plan.' },
                  { id: 'o2', label: 'Within a year', explanation: 'A relaxed monthly amount.' },
                ],
              },
              goal: null,
              approvedGoal: null,
              pathways: null,
              error: null,
              taskId: null,
            },
          ].filter((entry) => !removedGoals.has(entry.id))
        case 'workspace_tasks_state':
          return [
            {
              id: 'task-1',
              revision: 1,
              title: 'Run-walk: 20 minutes',
              parentGoalId: 'goal-fit',
              class: 'health',
              createdMs: Date.now() - 86_400_000,
              schedule: {
                local: '2026-10-30T08:00',
                zone: 'Local',
                repeat: 'daily',
                nextLocal: '2026-10-30T08:00',
                nextMs: Date.now() + 86_400_000,
              },
              paused: false,
              executionMode: 'manual',
              occurrences: [
                {
                  id: 'occ-1',
                  dueMs: Date.now() + 3_600_000,
                  state: 'pending',
                  runId: null,
                  reported: false,
                  completedMs: null,
                },
              ],
              error: null,
              completedSteps: 0,
              totalSteps: 0,
              instructions: 'Alternate 1 minute running with 2 minutes walking.',
            },
          ]
        case 'personalities_state':
          return {
            folder: 'C:/Users/me/.rune/personalities',
            active: { id: 'base', name: 'Rune', mark: 'companion' },
            entries: [
              {
                id: 'base',
                name: 'Rune',
                mark: 'companion',
                categories: [],
                prompt: 'Clear, calm and supportive. Keep answers short.',
              },
              {
                id: 'fitness-coach',
                name: 'Fitness coach',
                mark: 'fitness',
                categories: ['health', 'running'],
                prompt: 'Encourage steady progress. Celebrate small wins and avoid pressure.',
              },
            ],
          }
        case 'workspace_task_zone':
          return 'Local'
        case 'workspace_goal_modes':
          return ['manual', 'auto']
        case 'workspace_new_goal':
          return 'goal-new'
        case 'connections_state':
          return {
            revision: 1,
            activeId: 'local',
            connections: [
              {
                id: 'local',
                name: 'Model connection',
                baseUrl: 'http://127.0.0.1:8080',
                active: true,
                hasApiKey: false,
                selectedModels: ['gemma-4-12b-it', 'qwen3-vl'],
              },
              {
                id: 'cloud',
                name: 'Cloud provider',
                baseUrl: 'https://api.example.com/v1',
                active: false,
                hasApiKey: true,
              },
            ],
          }
        case 'get_control_label':
          return null
        case 'check_backend':
          return {
            revision: Date.now(),
            status: {
              reachable: true,
              authOk: true,
              missingModels: [],
              modelCapabilities: [],
              latencyMs: 12,
              error: null,
            },
          }
        case 'analysis_state':
          return new URLSearchParams(location.search).get('demo') === 'analysis'
            ? {
                ...analysisFixture(),
                id: 'insight-preview',
                monitor: { ...monitor(), width: window.innerWidth, height: window.innerHeight },
              }
            : analysisFixture()
        case 'installed_versions':
          return { studio: '0.0.3', cli: '0.0.3' }
        case 'video_recording_state':
          return new URLSearchParams(location.search).get('demo') === 'recording'
        case 'screen_recording_state':
          return { id: 0, phase: 'idle', seconds: 0, monitor: null, path: null, error: null }
        case 'screen_insight_targets':
          return [
            { target: { kind: 'screen', id: 1 }, label: 'Screen 1 — 1920×1080 (primary)' },
            { target: { kind: 'window', id: 2 }, label: 'Window — Project notes' },
          ]
        case 'overlay_info':
          return {
            x: 0,
            y: 0,
            mode:
              new URLSearchParams(location.search).get('demo') === 'analysis'
                ? 'analysis'
                : new URLSearchParams(location.search).has('demo')
                  ? 'control'
                  : 'none',
            primary: true,
            width: window.innerWidth,
            height: window.innerHeight,
            scaleFactor: 1,
          }
        case 'has_api_key':
          return true
        case 'debug_capture':
          return {
            monitor: monitor(),
            width: 1280,
            height: 720,
            bytes: 110_592,
            millis: 71,
            thumbnail: placeholderThumbnail(),
          }
        case 'debug_click':
          return { monitor: monitor(), pixelX: 960, pixelY: 540, cursorX: 960, cursorY: 540 }
        case 'plugin:app|version':
          return '0.0.3'
        case 'plugin:updater|check':
          return new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  rid: 1,
                  currentVersion: '0.0.3',
                  version: '0.0.4',
                  date: null,
                  body: 'Preview update',
                  rawJson: {},
                }),
              800,
            ),
          )
        case 'plugin:updater|download_and_install':
          return undefined
        default:
          return undefined
      }
    },
    { shouldMockEvents: true },
  )
}