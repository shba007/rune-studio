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
import { taskTimeline } from '@/config/taskTimeline'

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
  const history = shallowRef<unknown>(null)
  const timeline = computed(() => taskTimeline(history.value))
  type Media = {
    head: FlightHeader
    steps: FlightStep[]
    events: FlightEvent[]
    calls: FlightCall[]
    url: string | null
    note: string | null
  }
  let mediaRuns: Media[] = []
  const mediaHeader = shallowRef<FlightHeader | null>(null)
  const mediaOffset = computed(() =>
    timeline.value && mediaHeader.value
      ? Math.max(0, Date.parse(mediaHeader.value.startedUtc) - timeline.value.origin)
      : 0,
  )
  const mediaDuration = computed(() =>
    mediaHeader.value
      ? totalMs(stepStarts(mediaHeader.value.startedUtc, steps.value), mediaHeader.value)
      : 0,
  )

  function selectMediaAt(ms: number) {
    const origin = timeline.value?.origin ?? 0
    const offset = (media: Media) => Date.parse(media.head.startedUtc) - origin
    const eligible = mediaRuns.filter((media) => offset(media) <= ms)
    const selected =
      eligible.find((media) => ms <= offset(media) + (media.head.durationSecs ?? 0) * 1000) ??
      eligible.at(-1) ??
      mediaRuns[0]
    if (!selected || mediaHeader.value?.id === selected.head.id) return
    mediaHeader.value = selected.head
    steps.value = selected.steps
    events.value = selected.events
    calls.value = selected.calls
    videoUrl.value = selected.url
    videoNote.value = selected.note
  }

  const starts = computed(() =>
    stepStarts(mediaHeader.value?.startedUtc ?? header.value?.startedUtc ?? null, steps.value).map(
      (ms) => ms + mediaOffset.value,
    ),
  )
  const total = computed(
    () =>
      timeline.value?.total ??
      (header.value ? totalMs(starts.value, { durationSecs: header.value.durationSecs }) : 0),
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

  async function resolveVideo(
    head: FlightHeader,
  ): Promise<{ url: string | null; note: string | null }> {
    const video = head.video
    if (!video) {
      return { url: null, note: 'This task has no video; the screenshots are shown instead.' }
    }
    if (!video.file) {
      const note = video.unavailable
        ? `No video could be made on this computer (${video.unavailable}); the screenshots are shown instead.`
        : 'This task has no video; the screenshots are shown instead.'
      return { url: null, note }
    }
    try {
      const url = fileUrl(
        await invoke<string>('flight_file', { id: head.id, relative: video.file }),
      )
      return { url, note: null }
    } catch (error) {
      return {
        url: null,
        note: `The video file is missing (${problemText(error)}); the screenshots are shown instead.`,
      }
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
    history.value = null
    mediaRuns = []
    mediaHeader.value = null
    videoUrl.value = null
    steps.value = []
    events.value = []
    calls.value = []
    try {
      const head = await invoke<FlightHeader | null>('flight_run', { id })
      if (runId.value !== id) return
      if (!head) {
        header.value = null
        steps.value = []
        events.value = []
        calls.value = []
        problem.value = 'There is no such run in the record.'
        return
      }
      const records = await invoke<unknown>('flight_task_history', { id })
      if (runId.value !== id) return
      history.value = records
      const desktopIds =
        timeline.value?.rows
          .filter((task) => task.kind === 'desktop' && task.runId)
          .map((task) => task.runId!) ?? []
      const heads = desktopIds.length
        ? (
            await Promise.all(
              [...new Set(desktopIds)].map((runId) =>
                runId === id
                  ? Promise.resolve(head)
                  : invoke<FlightHeader | null>('flight_run', { id: runId }),
              ),
            )
          ).filter((item): item is FlightHeader => item !== null)
        : [head]
      const loaded = await Promise.all(
        heads.map(async (mediaHead) => {
          const [stepList, eventList, callList, video] = await Promise.all([
            invoke<FlightStep[]>('flight_steps', { id: mediaHead.id }),
            invoke<FlightEvent[]>('flight_events', { id: mediaHead.id }),
            invoke<FlightCall[]>('flight_calls', { id: mediaHead.id }),
            resolveVideo(mediaHead),
          ])
          return { head: mediaHead, steps: stepList, events: eventList, calls: callList, ...video }
        }),
      )
      if (runId.value !== id) return
      header.value = head
      mediaRuns = loaded.sort(
        (a, b) => Date.parse(a.head.startedUtc) - Date.parse(b.head.startedUtc),
      )
      selectMediaAt(0)
    } catch (error) {
      if (runId.value !== id) return
      header.value = null
      steps.value = []
      history.value = null
      problem.value = problemText(error)
    } finally {
      if (runId.value === id) loading.value = false
    }
  }

  async function fileOf(
    relative: string | null | undefined,
    id: string | null = mediaHeader.value?.id ?? runId.value,
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
      await invoke('flight_reveal', { id: mediaHeader.value?.id ?? runId.value })
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
    timeline,
    mediaHeader,
    mediaOffset,
    mediaDuration,
    selectMediaAt,
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