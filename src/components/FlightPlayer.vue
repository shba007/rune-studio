<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import FlightDrawer from '@/components/FlightDrawer.vue'
import FlightStage from '@/components/FlightStage.vue'
import FlightTimeline from '@/components/FlightTimeline.vue'
import { useFlight } from '@/composables/useFlight'
import {
  SPEEDS,
  actionLayer,
  caption,
  clock,
  eventsOfStep,
  faster,
  isOutsideAgent,
  nextTab,
  slower,
  stepAtTime,
  stepBadge,
  tabsFor,
  timeOfStep,
  typedTexts,
  type TabId,
} from '@/config/flight'

/**
 * The Flight Player: a finished (or still running) session replayed from the record. The video (or,
 * without one, the screenshots) with a light layer of what Rune did, a timeline of the steps, one
 * caption line, and a details drawer. It only reads: nothing here changes the record.
 */
const flight = useFlight()
const stage = ref<InstanceType<typeof FlightStage> | null>(null)

const timeMs = ref(0)
const playing = ref(false)
const rate = ref(1)
const drawerOpen = ref(false)
const tab = ref<TabId>('step')
const layerOn = ref(true)
const showTyped = ref(false)
const videoFailed = ref<string | null>(null)
const notice = ref<string | null>(null)
const still = ref<string | null>(null)
const stillMissing = ref(false)
const stepFile = ref<string | null>(null)

const header = flight.header
const steps = flight.steps
const hasVideo = computed(() => flight.videoUrl.value !== null && videoFailed.value === null)
const index = computed(() => stepAtTime(flight.starts.value, timeMs.value))
const step = computed(() => (index.value >= 0 ? steps.value[index.value] : undefined))
const typed = computed(() => typedTexts(steps.value))
const outside = computed(() => (header.value ? isOutsideAgent(header.value.source) : false))

const stepCalls = computed(() =>
  step.value ? flight.calls.value.filter((call) => call.step === step.value?.step) : [],
)
const stepEvents = computed(() =>
  header.value
    ? eventsOfStep(flight.events.value, header.value.startedUtc, steps.value, index.value)
    : [],
)
const retried = computed(() => {
  const set = new Set<number>()
  for (const call of flight.calls.value) {
    if (call.outcome !== 'retried' || call.step === null) continue
    const at = steps.value.findIndex((item) => item.step === call.step)
    if (at >= 0) set.add(at)
  }
  return set
})
const tabs = computed(() =>
  tabsFor({
    step: step.value,
    calls: stepCalls.value,
    hasFiles: step.value !== undefined,
    toolEvents: outside.value ? 1 : 0,
  }),
)
watch(tabs, (list) => {
  if (!list.includes(tab.value)) tab.value = list[0] ?? 'step'
})

const layer = computed(() => actionLayer(step.value, showTyped.value))
const line = computed(() => caption(step.value, showTyped.value))
const badge = computed(() => stepBadge(index.value, steps.value.length, step.value))

// ---- the picture for a step (when there is no video)
watch(
  () => step.value?.observationFile ?? null,
  async (file) => {
    stepFile.value = file
    still.value = null
    stillMissing.value = false
    if (!file) return
    const url = await flight.fileOf(file)
    if (stepFile.value !== file) return
    still.value = url
    stillMissing.value = url === null
  },
  { immediate: true },
)

// ---- the clock: the video's, or a virtual one over the stills
let frame = 0
let last = 0
function loop(now: number) {
  if (!playing.value || hasVideo.value) return
  timeMs.value = Math.min(flight.total.value, timeMs.value + (now - last) * rate.value)
  last = now
  if (timeMs.value >= flight.total.value) {
    playing.value = false
    return
  }
  frame = requestAnimationFrame(loop)
}

async function play() {
  if (!header.value || steps.value.length === 0) return
  if (timeMs.value >= flight.total.value - 50) await seek(0)
  playing.value = true
  if (hasVideo.value) {
    await stage.value?.play()
  } else {
    last = performance.now()
    frame = requestAnimationFrame(loop)
  }
}
async function pause() {
  playing.value = false
  if (frame) cancelAnimationFrame(frame)
  if (hasVideo.value) await stage.value?.pause()
}
function toggle() {
  return playing.value ? pause() : play()
}
async function seek(ms: number) {
  const next = Math.min(flight.total.value, Math.max(0, ms))
  timeMs.value = next
  if (hasVideo.value) await stage.value?.seek(next)
}
function goto(target: number) {
  return seek(timeOfStep(flight.starts.value, target))
}
function previousStep() {
  // Within the first moment of a step, go to the one before; otherwise back to its start.
  const here = index.value
  const atStart = timeMs.value - timeOfStep(flight.starts.value, here) < 600
  return goto(atStart ? here - 1 : here)
}
function nextStep() {
  return goto(Math.min(steps.value.length - 1, index.value + 1))
}
async function setRate(value: number) {
  rate.value = value
  if (hasVideo.value) await stage.value?.setRate(value)
}

function onVideoFailed(message: string) {
  videoFailed.value = message
  playing.value = false
}

// ---- opening a run
async function openRun(id: string) {
  await pause()
  timeMs.value = 0
  videoFailed.value = null
  notice.value = null
  drawerOpen.value = false
  tab.value = 'step'
  await flight.open(id)
}
async function backToRuns() {
  await pause()
  flight.header.value = null
  flight.runId.value = null
  flight.problem.value = null
  await flight.loadRuns()
}
async function reveal() {
  notice.value = await flight.reveal()
}
async function closeWindow() {
  try {
    await getCurrentWindow().close()
  } catch {
    window.close()
  }
}

// ---- keyboard
function onKey(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null
  if (event.ctrlKey || event.metaKey || event.altKey) return
  if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return
  const key = event.key
  if (key === 'Escape') {
    if (drawerOpen.value) drawerOpen.value = false
    else void closeWindow()
  } else if (!header.value) {
    return
  } else if (key === ' ' || key === 'Spacebar') {
    if (target?.tagName === 'BUTTON') return
    void toggle()
  } else if (key === 'ArrowLeft') void previousStep()
  else if (key === 'ArrowRight') void nextStep()
  else if (key === 'j' || key === 'J') void setRate(slower(rate.value))
  else if (key === 'k' || key === 'K') void setRate(faster(rate.value))
  else if (key === 'd' || key === 'D') drawerOpen.value = !drawerOpen.value
  else if (key === 'a' || key === 'A') layerOn.value = !layerOn.value
  else if (key === 't' || key === 'T') {
    drawerOpen.value = true
    const following = nextTab(tabs.value, tab.value)
    if (following) tab.value = following
  } else return
  event.preventDefault()
}

let unlisten: (() => void) | null = null
onMounted(async () => {
  window.addEventListener('keydown', onKey)
  const asked = new URLSearchParams(location.search).get('run')
  await flight.loadRuns()
  if (asked) await openRun(asked)
  try {
    unlisten = await listen<{ runId?: string | null }>('flight_open', async (event) => {
      const id = event.payload?.runId
      if (id) await openRun(id)
      else await backToRuns()
    })
  } catch {
    // A plain browser has no event bus; the run in the address is all there is.
  }
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  if (frame) cancelAnimationFrame(frame)
  unlisten?.()
})

const duration = computed(() =>
  header.value?.durationSecs == null ? null : clock(header.value.durationSecs * 1000),
)
const dateText = computed(() => {
  const at = header.value ? Date.parse(header.value.startedUtc) : Number.NaN
  return Number.isFinite(at) ? new Date(at).toLocaleString() : ''
})
const outcomeClass = computed(() =>
  header.value?.outcome === 'failed'
    ? 'border-[#d71921]/60 text-[#ff6b72]'
    : 'border-white/30 text-zinc-200',
)
const verdictLine = computed(() => {
  const verdict = header.value?.verdict
  return verdict ? `${verdict.result}${verdict.evidence ? ` · ${verdict.evidence}` : ''}` : ''
})

function runDate(text: string | null): string {
  const at = text ? Date.parse(text) : Number.NaN
  return Number.isFinite(at) ? new Date(at).toLocaleString() : ''
}
</script>

<template>
  <main
    class="flex h-screen flex-col overflow-hidden bg-[#141414] text-zinc-100"
    data-testid="flight"
  >
    <!-- The picker: no run chosen yet, or the record could not be read. -->
    <section v-if="!header" class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-6">
      <h1 class="m-0 text-title uppercase">Flight Player</h1>
      <p v-if="flight.loading.value" class="m-0 text-zinc-400" data-testid="loading">Loading…</p>
      <p
        v-else-if="flight.problem.value"
        class="m-0 text-zinc-300"
        role="alert"
        data-testid="problem"
      >
        {{ flight.problem.value }}
      </p>
      <p
        v-else-if="flight.runsLoaded.value && flight.runs.value.length === 0"
        class="m-0 text-zinc-400"
        data-testid="empty"
      >
        Nothing has been recorded yet. Run a task and it will show up here.
      </p>
      <ul v-else class="m-0 flex list-none flex-col gap-1 p-0" aria-label="Recorded sessions">
        <li v-for="run in flight.runs.value" :key="run.id">
          <button
            type="button"
            class="flex w-full cursor-pointer items-center gap-3 rounded-xl border border-white/10 bg-transparent px-4 py-3 text-left text-zinc-100 hover:bg-white/5"
            data-testid="run-row"
            @click="openRun(run.id)"
          >
            <span class="min-w-0 flex-1 truncate text-body">{{ run.task }}</span>
            <span class="text-caption text-zinc-400 uppercase">{{ run.outcome ?? 'open' }}</span>
            <span class="text-label text-zinc-500">{{ run.steps }} steps</span>
            <span class="text-label text-zinc-500">{{ runDate(run.startedUtc) }}</span>
          </button>
        </li>
      </ul>
    </section>

    <template v-else>
      <header
        class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-white/10 px-4 py-2"
      >
        <button
          type="button"
          class="grid size-6 cursor-pointer place-items-center rounded-full text-zinc-300 hover:bg-white/10"
          aria-label="Back to the list of sessions"
          data-testid="back"
          @click="backToRuns"
        >
          <AppIcon name="back" />
        </button>
        <h1 class="m-0 min-w-0 flex-1 truncate text-body" :title="header.task" data-testid="task">
          {{ header.task }}
        </h1>
        <span
          v-if="flight.live.value"
          class="rounded-full border border-white/40 px-2 text-caption uppercase"
          data-testid="live"
          >Live</span
        >
        <span
          v-else
          class="rounded-full border px-2 text-caption uppercase"
          :class="outcomeClass"
          data-testid="outcome"
          >{{ header.outcome ?? 'ended' }}</span
        >
        <span
          v-if="verdictLine"
          class="max-w-56 truncate rounded-full border border-white/20 px-2 text-caption text-zinc-300 uppercase"
          :title="verdictLine"
          data-testid="verdict"
          >{{ header.verdict?.result }}</span
        >
        <p
          class="m-0 flex flex-wrap gap-x-3 text-caption text-zinc-400 uppercase"
          data-testid="meta"
        >
          <span>{{ outside ? header.source : 'Rune' }}</span>
          <span>{{ dateText }}</span>
          <span>Duration {{ duration ?? '—' }}</span>
          <span>Steps {{ header.steps }}</span>
          <span>Retries {{ header.retries }}</span>
          <span v-if="header.model">{{ header.model }}</span>
        </p>
      </header>

      <p v-if="steps.length === 0" class="m-0 p-6 text-zinc-400" data-testid="no-steps">
        This session ended before it took a single step, so there is nothing to replay.
      </p>

      <div v-else class="relative flex min-h-0 flex-1">
        <div
          class="flex min-h-0 min-w-0 flex-1 flex-col motion-safe:transition-[margin] motion-safe:duration-[220ms]"
          :class="drawerOpen ? 'mr-[380px]' : ''"
        >
          <FlightStage
            ref="stage"
            :video-src="hasVideo ? flight.videoUrl.value : null"
            :still="still"
            :still-missing="stillMissing"
            :layer="layer"
            :layer-on="layerOn"
            :badge="badge"
            :note="videoFailed ?? flight.videoNote.value"
            @time="timeMs = $event"
            @playing="playing = $event"
            @failed="onVideoFailed"
          />

          <div
            class="flex shrink-0 flex-wrap items-center gap-2 px-4 pt-2"
            role="toolbar"
            aria-label="Playback"
          >
            <button
              type="button"
              class="grid size-7 cursor-pointer place-items-center rounded-full border border-white/20 hover:bg-white/10"
              aria-label="Previous step"
              data-testid="prev"
              @click="previousStep"
            >
              <AppIcon name="prev" />
            </button>
            <button
              type="button"
              class="grid size-8 cursor-pointer place-items-center rounded-full bg-[#f1f1ef] text-[#111] hover:bg-white"
              :aria-label="playing ? 'Pause' : 'Play'"
              data-testid="play"
              @click="toggle"
            >
              <AppIcon :name="playing ? 'pause' : 'play'" />
            </button>
            <button
              type="button"
              class="grid size-7 cursor-pointer place-items-center rounded-full border border-white/20 hover:bg-white/10"
              aria-label="Next step"
              data-testid="next"
              @click="nextStep"
            >
              <AppIcon name="next" />
            </button>
            <span class="text-label text-zinc-300 tabular-nums" data-testid="time"
              >{{ clock(timeMs) }} / {{ clock(flight.total.value) }}</span
            >
            <span class="ml-2 flex gap-1" role="group" aria-label="Speed">
              <button
                v-for="speed in SPEEDS"
                :key="speed"
                type="button"
                class="h-6 cursor-pointer rounded-full border px-2 text-label"
                :class="
                  rate === speed
                    ? 'border-white/60 bg-white/10'
                    : 'border-white/15 text-zinc-400 hover:text-zinc-200'
                "
                :aria-pressed="rate === speed"
                @click="setRate(speed)"
              >
                {{ speed }}x
              </button>
            </span>
            <span class="ml-auto flex items-center gap-1">
              <button
                type="button"
                class="inline-flex h-6 cursor-pointer items-center gap-1 rounded-full border border-white/20 px-3 text-label uppercase hover:bg-white/10"
                :aria-pressed="layerOn"
                data-testid="layer-toggle"
                @click="layerOn = !layerOn"
              >
                <AppIcon :name="layerOn ? 'eye' : 'hide'" size="sm" /> Actions
              </button>
              <button
                type="button"
                class="h-6 cursor-pointer rounded-full border border-white/20 px-3 text-label uppercase hover:bg-white/10"
                :aria-pressed="showTyped"
                :title="
                  showTyped
                    ? 'Hide typed text'
                    : 'Typed text is hidden. Show it only if nobody else can see this window.'
                "
                data-testid="typed-toggle"
                @click="showTyped = !showTyped"
              >
                Typed text: {{ showTyped ? 'shown' : 'hidden' }}
              </button>
              <button
                v-if="hasVideo"
                type="button"
                class="grid size-6 cursor-pointer place-items-center rounded-full border border-white/20 hover:bg-white/10"
                aria-label="Full screen"
                @click="stage?.fullscreen()"
              >
                <AppIcon name="expand" size="sm" />
              </button>
              <button
                type="button"
                class="inline-flex h-6 cursor-pointer items-center gap-1 rounded-full border border-white/20 px-3 text-label uppercase hover:bg-white/10"
                :aria-pressed="drawerOpen"
                data-testid="details"
                @click="drawerOpen = !drawerOpen"
              >
                <AppIcon name="drawer" size="sm" /> Details
              </button>
            </span>
          </div>

          <FlightTimeline
            :segments="flight.segments.value"
            :steps="steps"
            :starts="flight.starts.value"
            :total="flight.total.value"
            :time-ms="timeMs"
            :events="flight.events.value"
            :run-started-utc="header.startedUtc"
            :retried="retried"
            @seek="seek"
          />

          <p
            class="m-0 min-h-10 shrink-0 px-4 py-2 text-body text-zinc-300"
            aria-live="polite"
            data-testid="caption"
          >
            {{ line }}
          </p>
          <p v-if="notice" class="m-0 px-4 pb-2 text-status text-zinc-400" role="status">
            {{ notice }}
          </p>
        </div>

        <FlightDrawer
          :open="drawerOpen"
          :tabs="tabs"
          :tab="tab"
          :step="step"
          :index="index"
          :header="header"
          :typed="typed"
          :calls="stepCalls"
          :events="stepEvents"
          :show="showTyped"
          :screenshot="still"
          :load-bodies="flight.bodies"
          @update:tab="tab = $event"
          @close="drawerOpen = false"
          @toggle-show="showTyped = !showTyped"
          @reveal="reveal"
        />
      </div>
    </template>
  </main>
</template>