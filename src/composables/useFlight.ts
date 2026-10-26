import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { computed, ref, shallowRef } from 'vue'

import {
  segments as segmentsOf,
  stepStarts,
  totalMs,
  type FlightCall,
  type FlightEvent,
  type FlightHeader,
  type FlightRun,
  type FlightStep,
} from '@/config/flight'

export function fileUrl(path: string): string {
  return /^(data:|blob:|https?:|\/)/.test(path) ? path : convertFileSrc(path)
}

export function problemText(error: unknown): string {
  const text =
    typeof error === 'string' ? error : error instanceof Error ? error.message : String(error)
  return text.trim() === '' ? 'Something went wrong while reading the record.' : text
}

export function useFlight() {
  const runs = ref<FlightRun[]>([])
  const runsLoaded = ref(false)
  const runId = ref<string | null>(null)
  const header = shallowRef<FlightHeader | null>(null)
  const steps = shallowRef<FlightStep[]>([])
  const events = shallowRef<FlightEvent[]>([])
  const calls = shallowRef<FlightCall[]>([])
  const loading = ref(false)
  const problem = ref<string | null>(null)
  const videoUrl = ref<string | null>(null)
  const videoNote = ref<string | null>(null)

  const starts = computed(() => stepStarts(header.value?.startedUtc ?? null, steps.value))
  const total = computed(() =>
    header.value ? totalMs(starts.value, { durationSecs: header.value.durationSecs }) : 0,
  )
  const segments = computed(() => segmentsOf(starts.value, total.value))
  const live = computed(() => header.value !== null && header.value.endedUtc === null)

  async function loadRuns(): Promise<void> {
    try {
      runs.value = await invoke<FlightRun[]>('flight_runs', { limit: 60 })
      problem.value = null
    } catch (error) {
      runs.value = []
      problem.value = problemText(error)
    } finally {
      runsLoaded.value = true
    }
  }

  async function resolveVideo(head: FlightHeader): Promise<void> {
    videoUrl.value = null
    videoNote.value = null
    const video = head.video
    if (!video) {
      videoNote.value = 'This session has no video; the screenshots are shown instead.'
      return
    }
    if (!video.file) {
      videoNote.value = video.unavailable
        ? `No video could be made on this computer (${video.unavailable}); the screenshots are shown instead.`
        : 'This session has no video; the screenshots are shown instead.'
      return
    }
    try {
      videoUrl.value = fileUrl(
        await invoke<string>('flight_file', { id: head.id, relative: video.file }),
      )
    } catch (error) {
      videoNote.value = `The video file is missing (${problemText(error)}); the screenshots are shown instead.`
    }
  }

  async function remove(id: string): Promise<void> {
    const warning = await invoke<string | null>('flight_delete', { id })
    runs.value = runs.value.filter((run) => run.id !== id)
    problem.value = warning
  }

  async function open(id: string): Promise<void> {
    loading.value = true
    problem.value = null
    runId.value = id
    try {
      const head = await invoke<FlightHeader | null>('flight_run', { id })
      if (!head) {
        header.value = null
        steps.value = []
        events.value = []
        calls.value = []
        problem.value = 'There is no such run in the record.'
        return
      }
      const [stepList, eventList, callList] = await Promise.all([
        invoke<FlightStep[]>('flight_steps', { id }),
        invoke<FlightEvent[]>('flight_events', { id }),
        invoke<FlightCall[]>('flight_calls', { id }),
      ])
      header.value = head
      steps.value = stepList
      events.value = eventList
      calls.value = callList
      await resolveVideo(head)
    } catch (error) {
      header.value = null
      steps.value = []
      problem.value = problemText(error)
    } finally {
      loading.value = false
    }
  }

  async function fileOf(
    relative: string | null | undefined,
    id: string | null = runId.value,
  ): Promise<string | null> {
    if (!relative || !id) return null
    try {
      return fileUrl(await invoke<string>('flight_file', { id, relative }))
    } catch {
      return null
    }
  }

  async function bodies(callId: number): Promise<{ request: string; response: string } | null> {
    try {
      const pair = await invoke<[string, string] | null>('flight_call_body', { callId })
      return pair ? { request: pair[0], response: pair[1] } : null
    } catch {
      return null
    }
  }

  async function reveal(): Promise<string | null> {
    if (!runId.value) return null
    try {
      await invoke('flight_reveal', { id: runId.value })
      return null
    } catch (error) {
      return problemText(error)
    }
  }

  return {
    runs,
    runsLoaded,
    runId,
    header,
    steps,
    events,
    calls,
    loading,
    problem,
    videoUrl,
    videoNote,
    starts,
    total,
    segments,
    live,
    loadRuns,
    remove,
    open,
    fileOf,
    bodies,
    reveal,
  }
}

export type Flight = ReturnType<typeof useFlight>