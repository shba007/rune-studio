<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import TitleBar from '@/components/TitleBar.vue'
import WhenText from '@/components/WhenText.vue'
import FlightDrawer from '@/components/FlightDrawer.vue'
import FlightStage from '@/components/FlightStage.vue'
import FlightTimeline from '@/components/FlightTimeline.vue'
import { useFlight } from '@/composables/useFlight'
import { durationText } from '@/config/artifacts'
import {
  SPEEDS,
  actionLayer,
  clock,
  eventsOfStep,
  faster,
  isOutsideAgent,
  nextTab,
  pictureFileOf,
  pointerAt,
  slower,
  stepAtTime,
  stepBadge,
  tabsFor,
  timeOfStep,
  typedTexts,
  type TabId,
} from '@/config/flight'

const flight = useFlight()
const stage = ref<InstanceType<typeof FlightStage> | null>(null)

const timeMs = ref(0)
const playing = ref(false)
const rate = ref(1)
const drawerOpen = ref(false)
const tab = ref<TabId>('step')
const developer = ref(false)
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
  }).filter((id) => developer.value || (id !== 'api' && id !== 'prompt')),
)
watch(tabs, (list) => {
  if (!list.includes(tab.value)) tab.value = list[0] ?? 'step'
})

const layer = computed(() => actionLayer(step.value, showTyped.value))
const badge = computed(() => stepBadge(index.value, steps.value.length, step.value))

const cursor = computed(() => (index.value >= 0 ? pointerAt(steps.value, index.value) : null))
const pictureFile = computed(() => pictureFileOf(steps.value, index.value, outside.value))

watch(
  pictureFile,
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

async function openRun(id: string) {
  await pause()
  timeMs.value = 0
  videoFailed.value = null
  notice.value = null
  drawerOpen.value = false
  tab.value = 'step'
  await flight.open(id)
}
async function back() {
  await pause()
  try {
    await invoke('open_settings_window', { section: 'artifacts' })
  } catch {
    notice.value = null
  }
  await closeWindow()
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
  try {
    developer.value =
      (await invoke<{ developerMode?: boolean }>('get_settings')).developerMode === true
  } catch {
    developer.value = false
  }
  if (asked) await openRun(asked)
  else await back()
  try {
    unlisten = await listen<{ runId?: string | null }>('flight_open', async (event) => {
      const id = event.payload?.runId
      if (id) await openRun(id)
      else await back()
    })
  } catch {}
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
  if (frame) cancelAnimationFrame(frame)
  unlisten?.()
})

const duration = computed(() =>
  header.value?.durationSecs == null ? null : durationText(header.value.durationSecs),
)
const outcomeClass = computed(() =>
  header.value?.outcome === 'failed' ? 'text-[#ff6b72]' : 'text-zinc-100',
)
const outcomeDot = computed(() =>
  header.value?.outcome === 'failed'
    ? 'bg-[#d71921]'
    : header.value?.outcome === 'succeeded'
      ? 'bg-[#f1f1ef]'
      : 'border border-zinc-400',
)
const verdictLine = computed(() => {
  const verdict = header.value?.verdict
  return verdict ? `${verdict.result}${verdict.evidence ? ` · ${verdict.evidence}` : ''}` : ''
})
</script>

<template>
  <main class="flex h-screen flex-col overflow-hidden bg-black text-zinc-100" data-testid="flight">
    <TitleBar title="Rune Preview" />
    <section v-if="!header" class="flex min-h-0 flex-1 flex-col gap-3 p-6">
      <p v-if="flight.loading.value" class="m-0 text-zinc-400" data-testid="loading">Loading…</p>
      <p
        v-else-if="flight.problem.value"
        class="m-0 text-zinc-300"
        role="alert"
        data-testid="problem"
      >
        {{ flight.problem.value }}
      </p>
    </section>

    <template v-else>
      <header
        class="flex shrink-0 flex-wrap items-center gap-x-5 gap-y-2 border-b border-white/10 px-5 py-3"
      >
        <button
          type="button"
          class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full border-0 bg-transparent text-zinc-300 hover:bg-white/10"
          aria-label="Back to Sessions"
          data-testid="back"
          @click="back"
        >
          <AppIcon name="back" />
        </button>
        <div class="flex min-w-0 flex-1 flex-col">
          <h1 class="m-0 truncate text-title" :title="header.task" data-testid="task">
            {{ header.task }}
          </h1>
          <p class="m-0 flex items-center gap-2 text-caption text-zinc-500 uppercase">
            <span
              v-if="flight.live.value"
              class="flex items-center gap-1 text-zinc-100"
              data-testid="live"
              ><span class="size-1.5 animate-pulse rounded-full bg-[#d71921]" />Live</span
            >
            <span v-else class="flex items-center gap-1" :class="outcomeClass" data-testid="outcome"
              ><span class="size-1.5 rounded-full" :class="outcomeDot" />{{
                header.outcome ?? 'ended'
              }}</span
            >
            <template v-if="verdictLine">
              <span>·</span>
              <span class="max-w-56 truncate" :title="verdictLine" data-testid="verdict">{{
                header.verdict?.result
              }}</span>
            </template>
            <span>· <WhenText :iso="header.startedUtc" /></span>
          </p>
        </div>
        <dl class="m-0 flex flex-wrap gap-x-6 gap-y-1 text-caption uppercase" data-testid="meta">
          <div class="flex flex-col">
            <dt class="text-zinc-500">Duration</dt>
            <dd class="m-0 text-zinc-100 tabular-nums">{{ duration ?? '—' }}</dd>
          </div>
          <div class="flex flex-col">
            <dt class="text-zinc-500">Steps</dt>
            <dd class="m-0 text-zinc-100 tabular-nums">{{ header.steps }}</dd>
          </div>
          <div class="flex flex-col">
            <dt class="text-zinc-500">Retries</dt>
            <dd class="m-0 text-zinc-100 tabular-nums">{{ header.retries }}</dd>
          </div>
          <div v-if="outside" class="flex flex-col">
            <dt class="text-zinc-500">Agent</dt>
            <dd class="m-0 text-zinc-100 tabular-nums">{{ header.source }}</dd>
          </div>
          <div v-if="!outside && header.model" class="flex flex-col">
            <dt class="text-zinc-500">Model</dt>
            <dd class="m-0 text-zinc-100 tabular-nums">{{ header.model }}</dd>
          </div>
        </dl>
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
            :layer-on="true"
            :cursor="cursor"
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
                class="inline-flex h-6 cursor-pointer items-center gap-2 rounded-full border border-white/20 bg-transparent px-3 text-label text-zinc-100 uppercase hover:bg-white/10"
                :aria-pressed="!showTyped"
                title="Hide what was typed (passwords, messages) in the player"
                data-testid="typed-toggle"
                @click="showTyped = !showTyped"
              >
                <span
                  class="size-1.5 rounded-full"
                  :class="showTyped ? 'border border-zinc-400' : 'bg-[#f1f1ef]'"
                />
                Mask secrets
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
          :developer="developer"
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