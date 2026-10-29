<script setup lang="ts">
import RuneSegment from './RuneSegment.vue'
import RuneIconButton from './RuneIconButton.vue'
import RuneButton from './RuneButton.vue'
import ModelIcon from './ModelIcon.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import TitleBar from '@/components/TitleBar.vue'
import WhenText from '@/components/WhenText.vue'
import FlightDrawer from '@/components/FlightDrawer.vue'
import FlightStage from '@/components/FlightStage.vue'
import FlightTimeline from '@/components/FlightTimeline.vue'
import FlightAnalysis from '@/components/FlightAnalysis.vue'
import FlightTaskTimeline from '@/components/FlightTaskTimeline.vue'
import { problemText, useFlight } from '@/composables/useFlight'
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

const props = defineProps<{
  embedded?: boolean
  taskId?: string
  parentGoal?: { id: string; title: string } | null
}>()
const emit = defineEmits<{ back: []; goal: [id: string] }>()
const flight = useFlight()
const attempts = ref<string[]>([])
const selectedAttempt = ref('')
async function openTask(id: string) {
  try {
    const ids = await invoke<string[]>('task_attempts', { id })
    attempts.value = Array.isArray(ids) && ids.length ? ids : [id]
  } catch {
    attempts.value = [id]
  }
  selectedAttempt.value = attempts.value.at(-1) ?? id
  await openRun(selectedAttempt.value)
}
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
const grouped = computed(() => flight.timeline.value !== null)
const mediaCovered = computed(
  () =>
    !grouped.value ||
    (timeMs.value >= flight.mediaOffset.value &&
      timeMs.value <= flight.mediaOffset.value + flight.mediaDuration.value),
)
const hasVideo = computed(
  () => mediaCovered.value && flight.videoUrl.value !== null && videoFailed.value === null,
)
const playbackNote = computed(() => {
  if (
    grouped.value &&
    !flight.timeline.value?.rows.some((task) => task.kind === 'desktop' && task.runId)
  )
    return 'This request contains no recorded desktop task. Reasoning tasks have no screen capture.'
  if (!mediaCovered.value) return 'No desktop capture during this part of the request.'
  return videoFailed.value ?? flight.videoNote.value
})
const index = computed(() =>
  mediaCovered.value ? stepAtTime(flight.starts.value, timeMs.value) : -1,
)
const step = computed(() => (index.value >= 0 ? steps.value[index.value] : undefined))
const typed = computed(() => typedTexts(steps.value))
const outside = computed(() => (header.value ? isOutsideAgent(header.value.source) : false))

const stepCalls = computed(() =>
  step.value ? flight.calls.value.filter((call) => call.step === step.value?.step) : [],
)
const stepEvents = computed(() =>
  header.value
    ? eventsOfStep(
        flight.events.value,
        flight.mediaHeader.value?.startedUtc ?? header.value.startedUtc,
        steps.value,
        index.value,
      )
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
const pictureFile = computed(() =>
  mediaCovered.value ? pictureFileOf(steps.value, index.value, outside.value) : null,
)
watch(
  () => flight.mediaHeader.value?.id,
  () => {
    videoFailed.value = null
  },
)

watch([() => flight.mediaHeader.value?.id, hasVideo], async () => {
  if (!grouped.value) return
  await nextTick()
  if (hasVideo.value) {
    await stage.value?.seek(Math.max(0, timeMs.value - flight.mediaOffset.value))
    await stage.value?.setRate(rate.value)
    if (playing.value) await stage.value?.play()
  }
})
function onMediaTime(ms: number) {
  if (!grouped.value) timeMs.value = ms
}
function onMediaPlaying(value: boolean) {
  if (!grouped.value) playing.value = value
}

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
  if (!playing.value || (hasVideo.value && !grouped.value)) return
  timeMs.value = Math.min(flight.total.value, timeMs.value + (now - last) * rate.value)
  last = now
  if (grouped.value) flight.selectMediaAt(timeMs.value)
  if (timeMs.value >= flight.total.value) {
    playing.value = false
    if (hasVideo.value && grouped.value) void stage.value?.pause()
    return
  }
  frame = requestAnimationFrame(loop)
}

async function play() {
  if (!header.value || (steps.value.length === 0 && !grouped.value)) return
  if (timeMs.value >= flight.total.value - 50) await seek(0)
  playing.value = true
  if (hasVideo.value && !grouped.value) {
    await stage.value?.play()
  } else {
    last = performance.now()
    if (hasVideo.value) await stage.value?.play()
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
  if (grouped.value) {
    flight.selectMediaAt(next)
    await nextTick()
  }
  if (hasVideo.value) await stage.value?.seek(next - flight.mediaOffset.value)
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
  if (props.embedded) {
    emit('back')
    return
  }
  try {
    await invoke('open_workspace', { section: 'tasks' })
  } catch (error) {
    notice.value = problemText(error)
    return
  }
  await closeWindow()
}
async function reveal() {
  notice.value = await flight.reveal()
}
async function closeWindow() {
  if (props.embedded) {
    await back()
    return
  }
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
let disposed = false
watch(
  () => props.taskId,
  (id) => {
    if (props.embedded && id) void openTask(id)
  },
)
onMounted(async () => {
  window.addEventListener('keydown', onKey)
  const asked = props.taskId ?? new URLSearchParams(location.search).get('run')
  try {
    developer.value =
      (await invoke<{ developerMode?: boolean }>('get_settings')).developerMode === true
  } catch {
    developer.value = false
  }
  if (disposed) return
  if (asked) await openTask(asked)
  else await back()
  if (props.embedded || disposed) return
  try {
    const off = await listen<{ runId?: string | null }>('flight_open', async (event) => {
      const id = event.payload?.runId
      if (id) await openTask(id)
      else await back()
    })
    if (disposed) off()
    else unlisten = off
  } catch {}
})
onBeforeUnmount(() => {
  disposed = true
  window.removeEventListener('keydown', onKey)
  if (frame) cancelAnimationFrame(frame)
  unlisten?.()
})

const duration = computed(() =>
  grouped.value
    ? durationText(flight.total.value / 1000)
    : header.value?.durationSecs == null
      ? null
      : durationText(header.value.durationSecs),
)
const outcomeClass = computed(() =>
  header.value?.outcome === 'failed' ? 'text-error' : 'text-rune-ink',
)
const outcomeDot = computed(() =>
  header.value?.outcome === 'failed'
    ? 'bg-attention'
    : header.value?.outcome === 'succeeded'
      ? 'bg-rune-ink'
      : 'bg-rune-raised',
)
const verdictLine = computed(() => {
  const verdict = header.value?.verdict
  return verdict ? `${verdict.result}${verdict.evidence ? ` · ${verdict.evidence}` : ''}` : ''
})
</script>

<template>
  <main
    class="flex min-h-0 flex-col overflow-hidden bg-black text-rune-ink"
    :class="embedded ? 'h-full flex-1' : 'h-screen'"
    data-testid="flight"
  >
    <TitleBar v-if="!embedded" title="Workspace" />
    <section v-if="!header" class="flex min-h-0 flex-1 flex-col gap-3 p-6">
      <RuneButton variant="ghost" size="sm" class="self-start" @click="back">
        Back to Tasks
      </RuneButton>
      <p v-if="flight.loading.value" class="m-0 text-rune-text-2" data-testid="loading">Loading…</p>
      <p
        v-else-if="flight.problem.value"
        class="m-0 text-rune-text-2"
        role="alert"
        data-testid="problem"
      >
        {{ flight.problem.value }}
      </p>
    </section>

    <template v-else>
      <header class="flex shrink-0 flex-wrap items-center gap-x-5 gap-y-2 px-5 py-3">
        <RuneIconButton
          variant="ghost"
          size="sm"
          label="Back to Tasks"
          data-testid="back"
          @click="back"
        >
          <AppIcon name="back" />
        </RuneIconButton>
        <div class="flex min-w-0 flex-1 flex-col">
          <select
            v-if="attempts.length > 1"
            v-model="selectedAttempt"
            aria-label="Execution attempt"
            class="mb-2 self-start rounded-rune-tile bg-rune-raised px-2 py-1 text-rune-caption text-rune-ink"
            @change="openRun(selectedAttempt)"
          >
            <option v-for="(id, index) in attempts" :key="id" :value="id">
              Attempt {{ index + 1 }}{{ index === attempts.length - 1 ? ' · Latest' : '' }}
            </option>
          </select>
          <RuneButton
            v-if="parentGoal"
            variant="link"
            class="max-w-xs truncate"
            @click="emit('goal', parentGoal.id)"
          >
            ← {{ parentGoal.title }}
          </RuneButton>
          <h1 class="m-0 truncate text-rune-title" :title="header.task" data-testid="task">
            {{ header.task }}
          </h1>
          <p class="m-0 flex items-center gap-2 text-rune-heading text-rune-text-2 uppercase">
            <span
              v-if="flight.live.value"
              class="flex items-center gap-1 text-rune-ink"
              data-testid="live"
              ><span
                class="size-1.5 motion-safe:animate-pulse rounded-full bg-attention"
              />Live</span
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
        <dl
          class="m-0 flex flex-wrap gap-x-6 gap-y-1 text-rune-heading uppercase"
          data-testid="meta"
        >
          <div class="flex flex-col">
            <dt class="text-rune-text-2">Duration</dt>
            <dd class="m-0 text-rune-ink tabular-nums">{{ duration ?? '—' }}</dd>
          </div>
          <div class="flex flex-col">
            <dt class="text-rune-text-2">Steps</dt>
            <dd class="m-0 text-rune-ink tabular-nums">{{ header.steps }}</dd>
          </div>
          <div class="flex flex-col">
            <dt class="text-rune-text-2">Retries</dt>
            <dd class="m-0 text-rune-ink tabular-nums">{{ header.retries }}</dd>
          </div>
          <div v-if="outside" class="flex flex-col">
            <dt class="text-rune-text-2">Agent</dt>
            <dd class="m-0 text-rune-ink tabular-nums">{{ header.source }}</dd>
          </div>
          <div v-if="!outside && header.model" class="flex flex-col">
            <dt class="text-rune-text-2">Model</dt>
            <dd class="m-0 flex items-center gap-2 text-rune-ink tabular-nums">
              <ModelIcon :model="header.model" />{{ header.model }}
            </dd>
          </div>
        </dl>
      </header>

      <FlightAnalysis
        v-if="header.source === 'screen-insight'"
        :id="header.id"
        :analysis="header.analysis"
        :summary="header.summary"
      />
      <p
        v-else-if="steps.length === 0 && !grouped"
        class="m-0 p-6 text-rune-text-2"
        data-testid="no-steps"
      >
        This task ended before it took a single step, so there is nothing to replay.
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
            :note="playbackNote"
            @time="onMediaTime"
            @playing="onMediaPlaying"
            @failed="onVideoFailed"
          />

          <div
            class="flex shrink-0 flex-wrap items-center gap-2 px-4 pt-2"
            role="toolbar"
            aria-label="Playback"
          >
            <RuneIconButton
              size="sm"
              label="Previous step"
              data-testid="prev"
              @click="previousStep"
            >
              <AppIcon name="prev" />
            </RuneIconButton>
            <RuneIconButton
              variant="primary"
              size="sm"
              :label="playing ? 'Pause' : 'Play'"
              data-testid="play"
              @click="toggle"
            >
              <AppIcon :name="playing ? 'pause' : 'play'" />
            </RuneIconButton>
            <RuneIconButton size="sm" label="Next step" data-testid="next" @click="nextStep">
              <AppIcon name="next" />
            </RuneIconButton>
            <span class="text-rune-caption text-rune-text-2 tabular-nums" data-testid="time"
              >{{ clock(timeMs) }} / {{ clock(flight.total.value) }}</span
            >
            <span class="ml-2 flex gap-1" role="group" aria-label="Speed">
              <RuneSegment
                v-for="speed in SPEEDS"
                :key="speed"
                :selected="rate === speed"
                :aria-pressed="rate === speed"
                @click="setRate(speed)"
              >
                {{ speed }}x
              </RuneSegment>
            </span>
            <span class="ml-auto flex items-center gap-1">
              <RuneButton
                variant="ghost"
                size="sm"
                :aria-pressed="!showTyped"
                title="Hide what was typed (passwords, messages) in the player"
                data-testid="typed-toggle"
                @click="showTyped = !showTyped"
              >
                <span
                  class="size-1.5 rounded-full"
                  :class="showTyped ? 'bg-rune-raised' : 'bg-rune-ink'"
                />
                Mask secrets
              </RuneButton>
              <RuneIconButton
                size="sm"
                v-if="hasVideo"
                label="Full screen"
                @click="stage?.fullscreen()"
              >
                <AppIcon name="expand" size="sm" />
              </RuneIconButton>
              <RuneButton
                size="sm"
                :aria-pressed="drawerOpen"
                data-testid="details"
                @click="drawerOpen = !drawerOpen"
              >
                <AppIcon name="drawer" size="sm" /> Details
              </RuneButton>
            </span>
          </div>

          <FlightTaskTimeline
            v-if="flight.timeline.value"
            :timeline="flight.timeline.value"
            :time-ms="timeMs"
            @seek="seek"
          />
          <FlightTimeline
            v-else
            :segments="flight.segments.value"
            :steps="steps"
            :starts="flight.starts.value"
            :total="flight.total.value"
            :time-ms="timeMs"
            :events="flight.events.value"
            :run-started-utc="flight.mediaHeader.value?.startedUtc ?? header.startedUtc"
            :retried="retried"
            @seek="seek"
          />

          <p v-if="notice" class="m-0 px-4 pb-2 text-rune-caption text-rune-text-2" role="status">
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