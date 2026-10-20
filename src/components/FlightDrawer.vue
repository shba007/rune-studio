<script setup lang="ts">
import { useClipboard } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import {
  TAB_LABEL,
  argsText,
  clock,
  isOutsideAgent,
  maskTyped,
  phaseTimes,
  pretty,
  reasoningOf,
  visibleRows,
  type FlightCall,
  type FlightEvent,
  type FlightHeader,
  type FlightStep,
  type TabId,
} from '@/config/flight'

/**
 * The details of the current step, one calm tab at a time. A tab is listed only when the record has
 * something for it. RAW shows the stored JSON with a Copy button. Typed text stays hidden behind SHOW.
 */
const props = defineProps<{
  open: boolean
  tabs: TabId[]
  tab: TabId
  step: FlightStep | undefined
  index: number
  header: FlightHeader
  /** Every step of the run, so text typed anywhere is masked wherever it shows up. */
  typed: string[]
  calls: FlightCall[]
  events: FlightEvent[]
  show: boolean
  screenshot: string | null
  loadBodies: (callId: number) => Promise<{ request: string; response: string } | null>
}>()
const emit = defineEmits<{
  'update:tab': [tab: TabId]
  close: []
  'toggle-show': []
  reveal: []
}>()

const raw = ref(false)
const { copy, copied } = useClipboard({ copiedDuring: 1500 })
const mask = (text: string | null | undefined) => maskTyped(text ?? '', props.typed, props.show)

const stepRaw = computed(() => {
  if (!props.step) return ''
  const copyOf = {
    ...props.step,
    argsJson: props.step.argsJson ? argsText(props.step, props.show) : null,
  }
  return mask(JSON.stringify(copyOf, null, 2))
})

// ---- API tab: fixed-height rows, only the ones in view are drawn
const ROW = 32
const listTop = ref(0)
const LIST_HEIGHT = 224
const rows = computed(() => visibleRows(props.calls.length, listTop.value, LIST_HEIGHT, ROW))
const shownCalls = computed(() => props.calls.slice(rows.value.from, rows.value.to))
const picked = ref<number | null>(null)
const pickedBodies = ref<{ request: string; response: string } | null>(null)
const bodiesState = ref<'idle' | 'loading' | 'missing'>('idle')
const pickedCall = computed(() => props.calls.find((call) => call.id === picked.value))

async function pick(call: FlightCall) {
  if (picked.value === call.id) {
    picked.value = null
    pickedBodies.value = null
    return
  }
  picked.value = call.id
  pickedBodies.value = null
  bodiesState.value = 'loading'
  const found = await props.loadBodies(call.id)
  if (picked.value !== call.id) return
  pickedBodies.value = found
  bodiesState.value = found ? 'idle' : 'missing'
}
watch(
  () => props.step?.step,
  () => {
    picked.value = null
    pickedBodies.value = null
    listTop.value = 0
  },
)

const reasoning = computed(() => {
  for (const call of props.calls) {
    if (call.kind !== 'decide') continue
    // Bodies are loaded on demand; the reasoning shows once the decide call is opened.
    if (call.id === picked.value && pickedBodies.value) {
      return reasoningOf(pickedBodies.value.response)
    }
  }
  return null
})

const timing = computed(() => (props.step ? phaseTimes(props.step.phases) : []))
const timingMax = computed(() => Math.max(1, ...timing.value.map((item) => item.ms)))
const riskReasons = computed(() => {
  try {
    const value: unknown = JSON.parse(props.step?.riskReasonsJson ?? '[]')
    return Array.isArray(value) ? value.map(String) : []
  } catch {
    return []
  }
})
const outside = computed(() => isOutsideAgent(props.header.source))
const asked = computed(() =>
  props.events.filter((event) => event.kind === 'ask_user' || event.kind === 'invalid_answer'),
)
const rawText = computed(() => {
  switch (props.tab) {
    case 'api':
      return mask(JSON.stringify(props.calls, null, 2))
    case 'safety':
      return mask(
        JSON.stringify(
          {
            riskStated: props.step?.riskStated,
            riskFinal: props.step?.riskFinal,
            reasons: riskReasons.value,
            gate: props.step?.gate,
            events: props.events,
          },
          null,
          2,
        ),
      )
    case 'timing':
      return JSON.stringify(props.step?.phases ?? {}, null, 2)
    default:
      return stepRaw.value
  }
})
</script>

<template>
  <aside
    class="absolute inset-y-0 right-0 z-10 flex w-[380px] max-w-full flex-col border-l border-white/10 bg-[#141414] text-zinc-100 motion-safe:transition-transform motion-safe:duration-[220ms]"
    :class="open ? 'translate-x-0' : 'translate-x-full'"
    :aria-hidden="!open"
    :inert="!open"
    aria-label="Step details"
    data-testid="drawer"
  >
    <header class="flex h-10 shrink-0 items-center gap-2 border-b border-white/10 px-3">
      <span class="text-caption text-zinc-400 uppercase">Details</span>
      <button
        type="button"
        class="ml-auto h-6 cursor-pointer rounded-full border border-white/20 px-3 text-label text-zinc-300 uppercase hover:bg-white/10"
        :aria-pressed="raw"
        data-testid="raw-toggle"
        @click="raw = !raw"
      >
        Raw
      </button>
      <button
        type="button"
        class="grid size-6 cursor-pointer place-items-center rounded-full text-zinc-300 hover:bg-white/10"
        aria-label="Close details"
        @click="emit('close')"
      >
        <AppIcon name="close" />
      </button>
    </header>

    <nav
      class="flex shrink-0 flex-wrap gap-1 border-b border-white/10 px-3 py-2"
      role="tablist"
      aria-label="Detail tabs"
    >
      <button
        v-for="id in tabs"
        :key="id"
        type="button"
        role="tab"
        :aria-selected="tab === id"
        class="h-6 cursor-pointer rounded-full border px-3 text-label uppercase transition"
        :class="
          tab === id
            ? 'border-white/60 bg-white/10 text-zinc-50'
            : 'border-white/15 text-zinc-400 hover:text-zinc-200'
        "
        :data-testid="`tab-${id}`"
        @click="emit('update:tab', id)"
      >
        {{ TAB_LABEL[id] }}
      </button>
    </nav>

    <div class="min-h-0 flex-1 overflow-y-auto p-3 text-body" role="tabpanel">
      <p v-if="!step" class="m-0 text-zinc-500">No step is selected.</p>

      <template v-else-if="raw">
        <div class="mb-2 flex justify-end">
          <button
            type="button"
            class="h-6 cursor-pointer rounded-full border border-white/20 px-3 text-label text-zinc-300 uppercase hover:bg-white/10"
            data-testid="copy"
            @click="copy(rawText)"
          >
            {{ copied ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <pre
          class="m-0 text-status break-words whitespace-pre-wrap text-zinc-300"
          data-testid="raw"
          >{{ rawText }}</pre>
      </template>

      <template v-else-if="tab === 'step'">
        <dl class="m-0 grid grid-cols-[88px_1fr] gap-x-3 gap-y-2">
          <dt class="text-caption text-zinc-500 uppercase">Step</dt>
          <dd class="m-0">{{ index + 1 }} of {{ header.steps }}</dd>
          <dt class="text-caption text-zinc-500 uppercase">Action</dt>
          <dd class="m-0">{{ (step.tool ?? 'none').replaceAll('_', ' ') }}</dd>
          <dt class="flex items-center gap-1 text-caption text-zinc-500 uppercase">Arguments</dt>
          <dd class="m-0 break-words" data-testid="args">
            {{ argsText(step, show) || '—' }}
            <button
              v-if="step.tool === 'type_text'"
              type="button"
              class="ml-1 h-5 cursor-pointer rounded-full border border-white/25 px-2 text-caption text-zinc-300 uppercase hover:bg-white/10"
              :aria-pressed="show"
              data-testid="show-typed"
              @click="emit('toggle-show')"
            >
              {{ show ? 'Hide' : 'Show' }}
            </button>
          </dd>
          <dt class="text-caption text-zinc-500 uppercase">Reason</dt>
          <dd class="m-0">{{ mask(step.reason) || '—' }}</dd>
          <dt class="text-caption text-zinc-500 uppercase">Result</dt>
          <dd class="m-0" :class="step.error ? 'text-[#ff6b72]' : ''">
            {{ step.result ?? '—' }}<template v-if="step.error"> · {{ mask(step.error) }}</template>
          </dd>
          <template v-if="step.goal !== null">
            <dt class="text-caption text-zinc-500 uppercase">Goal</dt>
            <dd class="m-0">Plan item {{ step.goal }}</dd>
          </template>
        </dl>
        <img
          v-if="screenshot"
          :src="screenshot"
          alt="What the model saw at this step"
          class="mt-3 w-full rounded-lg border border-white/10"
          data-testid="shot"
        />
      </template>

      <template v-else-if="tab === 'reasoning'">
        <p class="m-0 mb-2 text-zinc-400">
          <template v-if="step.thinkingChars !== null"
            >{{ step.thinkingChars }} characters of reasoning</template
          >
          <template v-if="step.completionTokens !== null">
            · {{ step.completionTokens }} tokens</template
          >
        </p>
        <pre
          v-if="reasoning"
          class="m-0 text-status break-words whitespace-pre-wrap text-zinc-300"
          >{{ mask(reasoning) }}</pre>
        <p v-else class="m-0 text-zinc-500">
          The reasoning text itself was not recorded for this step. Open the matching call in the
          API tab to look for it.
        </p>
        <h3 class="mt-4 mb-1 text-caption text-zinc-500 uppercase">Answer</h3>
        <p class="m-0">{{ mask(step.reason) || '—' }}</p>
      </template>

      <template v-else-if="tab === 'prompt'">
        <h3 class="mb-1 text-caption text-zinc-500 uppercase">Message sent to the model</h3>
        <pre
          class="m-0 text-status break-words whitespace-pre-wrap text-zinc-300"
          data-testid="prompt"
          >{{ mask(step.promptText) }}</pre>
      </template>

      <template v-else-if="tab === 'safety'">
        <dl class="m-0 grid grid-cols-[110px_1fr] gap-x-3 gap-y-2">
          <dt class="text-caption text-zinc-500 uppercase">Stated risk</dt>
          <dd class="m-0">{{ step.riskStated ?? '—' }}</dd>
          <dt class="text-caption text-zinc-500 uppercase">Final risk</dt>
          <dd class="m-0">{{ step.riskFinal ?? '—' }}</dd>
          <dt class="text-caption text-zinc-500 uppercase">Gate</dt>
          <dd class="m-0">{{ step.gate ?? '—' }}</dd>
          <template v-if="riskReasons.length">
            <dt class="text-caption text-zinc-500 uppercase">Reasons</dt>
            <dd class="m-0">{{ riskReasons.join('; ') }}</dd>
          </template>
        </dl>
        <template v-if="asked.length">
          <h3 class="mt-4 mb-1 text-caption text-zinc-500 uppercase">Questions and answers</h3>
          <ul class="m-0 list-none space-y-1 p-0 text-status text-zinc-300">
            <li v-for="event in asked" :key="event.id">
              {{ event.kind }} · {{ mask(event.dataJson) }}
            </li>
          </ul>
        </template>
      </template>

      <template v-else-if="tab === 'api'">
        <div
          class="max-h-56 overflow-y-auto rounded-lg border border-white/10"
          :style="{ height: `${Math.min(LIST_HEIGHT, calls.length * ROW)}px` }"
          data-testid="calls"
          @scroll="listTop = ($event.target as HTMLElement).scrollTop"
        >
          <div class="relative" :style="{ height: `${calls.length * ROW}px` }">
            <button
              v-for="(call, offset) in shownCalls"
              :key="call.id"
              type="button"
              class="absolute right-0 left-0 flex cursor-pointer items-center gap-2 border-0 border-b border-white/5 bg-transparent px-2 text-left text-status text-zinc-200 hover:bg-white/5"
              :class="picked === call.id ? 'bg-white/10' : ''"
              :style="{ top: `${(rows.from + offset) * ROW}px`, height: `${ROW}px` }"
              :aria-expanded="picked === call.id"
              data-testid="call"
              @click="pick(call)"
            >
              <span class="w-14 truncate">{{ call.kind }}</span>
              <span class="w-8">#{{ call.attempt }}</span>
              <span class="w-9">{{ call.status ?? '—' }}</span>
              <span class="w-14">{{ call.durationMs }} ms</span>
              <span v-if="call.waitBeforeMs > 0" class="w-14 text-zinc-400"
                >+{{ call.waitBeforeMs }} ms</span
              >
              <span
                class="ml-auto uppercase"
                :class="
                  call.outcome === 'failed' || call.outcome === 'timeout' ? 'text-[#ff6b72]' : ''
                "
                >{{ call.outcome }}</span
              >
            </button>
          </div>
        </div>
        <div v-if="pickedCall" class="mt-3 space-y-2" data-testid="bodies">
          <p v-if="pickedCall.error" class="m-0 text-status text-[#ff6b72]">
            {{ mask(pickedCall.error) }}
          </p>
          <p v-if="bodiesState === 'loading'" class="m-0 text-zinc-500">Loading…</p>
          <p v-else-if="bodiesState === 'missing'" class="m-0 text-zinc-500">
            The bodies of this call are not in the record.
          </p>
          <template v-else-if="pickedBodies">
            <p
              v-if="pickedCall.truncated"
              class="m-0 text-status text-zinc-400"
              data-testid="truncated"
            >
              This body was cut at 64 KB when it was stored.
            </p>
            <p class="m-0 text-status text-zinc-500">
              Images are not kept in the record (only their size); the screenshot of the step is
              shown above.
            </p>
            <h3 class="m-0 text-caption text-zinc-500 uppercase">Request</h3>
            <pre
              class="m-0 max-h-60 overflow-auto text-status break-words whitespace-pre-wrap text-zinc-300"
              >{{ mask(pretty(pickedBodies.request)) }}</pre>
            <h3 class="m-0 text-caption text-zinc-500 uppercase">Response</h3>
            <pre
              class="m-0 max-h-60 overflow-auto text-status break-words whitespace-pre-wrap text-zinc-300"
              >{{ mask(pretty(pickedBodies.response)) }}</pre>
          </template>
        </div>
      </template>

      <template v-else-if="tab === 'timing'">
        <ul class="m-0 list-none space-y-2 p-0">
          <li
            v-for="item in timing"
            :key="item.phase"
            class="grid grid-cols-[64px_1fr_64px] items-center gap-2 text-status"
          >
            <span class="text-zinc-400 uppercase">{{ item.phase }}</span>
            <span class="h-1.5 rounded-full bg-white/10"
              ><span
                class="block h-full rounded-full bg-zinc-200"
                :style="{ width: `${(item.ms / timingMax) * 100}%` }"
            /></span>
            <span class="text-right">{{ item.ms }} ms</span>
          </li>
        </ul>
        <p class="mt-4 mb-0 text-status text-zinc-400">
          Run:
          {{ header.durationSecs === null ? 'still open' : clock(header.durationSecs * 1000) }} ·
          {{ header.steps }} steps · {{ header.retries }} retried calls
        </p>
      </template>

      <template v-else-if="tab === 'tools'">
        <p v-if="outside" class="m-0">
          <strong class="font-semibold">rune_{{ step.tool }}</strong> called by {{ header.source }}
        </p>
        <pre class="m-0 mt-2 text-status break-words whitespace-pre-wrap text-zinc-300">{{
          argsText(step, show)
        }}</pre>
      </template>

      <template v-else-if="tab === 'files'">
        <ul class="m-0 list-none space-y-1 p-0 text-status text-zinc-300">
          <li v-if="step.observationFile">{{ step.observationFile }}</li>
          <li v-if="header.video?.file">{{ header.video.file }}</li>
          <li>session.json</li>
        </ul>
        <button
          type="button"
          class="mt-3 h-7 cursor-pointer rounded-full border border-white/25 px-4 text-label text-zinc-200 uppercase hover:bg-white/10"
          data-testid="reveal"
          @click="emit('reveal')"
        >
          Reveal in folder
        </button>
      </template>
    </div>
  </aside>
</template>