<script setup lang="ts">
import { useClipboard } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import { KIND_ICON, TOOL_LABEL, kindOf } from '@/config/actions'
import {
  TAB_LABEL,
  promptSections,
  riskLevel,
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

const props = defineProps<{
  open: boolean
  tabs: TabId[]
  tab: TabId
  step: FlightStep | undefined
  index: number
  header: FlightHeader
  typed: string[]
  calls: FlightCall[]
  events: FlightEvent[]
  show: boolean
  developer: boolean
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
const sections = computed(() =>
  promptSections(props.step?.promptText ? mask(props.step.promptText) : ''),
)
const RISK = ['Low', 'Medium', 'High']
const inputRows = computed(() => {
  if (!props.step?.argsJson) return []
  try {
    const args = JSON.parse(argsText(props.step, props.show)) as Record<string, unknown>
    return Object.entries(args)
      .filter(([key]) => !key.startsWith('_') && !['reason', 'risk', 'goal'].includes(key))
      .map(([key, value]) => ({
        key: key.replaceAll('_', ' '),
        value: typeof value === 'string' ? value : JSON.stringify(value),
      }))
  } catch {
    return [{ key: 'input', value: argsText(props.step, props.show) }]
  }
})
const toolLabel = computed(() => {
  const tool = props.step?.tool ?? ''
  return TOOL_LABEL[tool] ?? (tool || 'none').replaceAll('_', ' ')
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
    class="absolute inset-y-0 right-0 z-10 flex w-[380px] max-w-full flex-col border-l border-white/10 bg-black text-zinc-100 motion-safe:transition-transform motion-safe:duration-[220ms]"
    :class="open ? 'translate-x-0' : 'translate-x-full'"
    :aria-hidden="!open"
    :inert="!open"
    aria-label="Step details"
    data-testid="drawer"
  >
    <header class="flex h-10 shrink-0 items-center gap-2 border-b border-white/10 px-3">
      <span class="text-caption text-zinc-400 uppercase">Details</span>
      <button
        v-if="developer"
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
        :class="developer ? '' : 'ml-auto'"
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

      <template v-else-if="raw && developer">
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
        <div class="flex items-center gap-3">
          <span
            class="grid size-10 shrink-0 place-items-center rounded-full bg-white/8 text-zinc-100"
          >
            <AppIcon :name="KIND_ICON[kindOf(step.tool ?? '')]" />
          </span>
          <div class="flex min-w-0 flex-col">
            <span class="text-title">{{ toolLabel }}</span>
            <span class="text-caption text-zinc-500 uppercase"
              >Step {{ index + 1 }} of {{ header.steps
              }}<template v-if="step.goal !== null"> · Goal {{ step.goal }}</template></span
            >
          </div>
          <span
            class="ml-auto rounded-full border px-2 text-caption uppercase"
            :class="
              step.error ? 'border-[#d71921]/60 text-[#ff6b72]' : 'border-white/30 text-zinc-200'
            "
            >{{ step.error ? 'Error' : (step.result ?? '—') }}</span
          >
        </div>
        <p v-if="step.error" class="m-0 mt-3 text-label text-[#ff6b72]">{{ mask(step.error) }}</p>

        <div v-if="riskLevel(step.riskFinal ?? step.riskStated) >= 0" class="mt-4">
          <div class="mb-1 flex items-center justify-between text-caption uppercase">
            <span class="text-zinc-500">Risk</span>
            <span class="text-zinc-300">{{
              RISK[riskLevel(step.riskFinal ?? step.riskStated)]
            }}</span>
          </div>
          <div class="flex gap-1" aria-hidden="true">
            <span
              v-for="n in 3"
              :key="n"
              class="h-1 flex-1 rounded-full"
              :class="
                n - 1 <= riskLevel(step.riskFinal ?? step.riskStated)
                  ? n === 3
                    ? 'bg-[#d71921]'
                    : 'bg-zinc-200'
                  : 'bg-white/10'
              "
            />
          </div>
        </div>

        <figure v-if="mask(step.reason)" class="m-0 mt-4 border-l border-white/20 pl-3">
          <figcaption class="mb-1 text-caption text-zinc-500 uppercase">Why</figcaption>
          <p class="m-0 line-clamp-6 text-body leading-snug text-zinc-200">
            {{ mask(step.reason) }}
          </p>
        </figure>

        <div v-if="inputRows.length || step.tool === 'type_text'" class="mt-4">
          <div class="mb-1 flex items-center justify-between">
            <span class="text-caption text-zinc-500 uppercase">Input</span>
            <button
              v-if="step.tool === 'type_text'"
              type="button"
              class="h-5 cursor-pointer rounded-full border border-white/25 bg-transparent px-2 text-caption text-zinc-300 uppercase hover:bg-white/10"
              :aria-pressed="show"
              data-testid="show-typed"
              @click="emit('toggle-show')"
            >
              {{ show ? 'Hide' : 'Show' }}
            </button>
          </div>
          <dl class="m-0 flex flex-col gap-2 rounded-xl bg-white/5 px-3 py-2" data-testid="args">
            <div v-for="row in inputRows" :key="row.key" class="flex flex-col">
              <dt class="text-caption text-zinc-500 uppercase">{{ row.key }}</dt>
              <dd class="m-0 text-label break-words text-zinc-100">{{ row.value }}</dd>
            </div>
            <p v-if="inputRows.length === 0" class="m-0 text-label text-zinc-500">No input</p>
          </dl>
        </div>
        <img
          v-if="screenshot"
          :src="screenshot"
          alt="What the model saw at this step"
          class="mt-4 w-full rounded-xl border border-white/10"
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
        <p class="m-0 mb-2 text-caption text-zinc-500 uppercase">Sent to the model</p>
        <div class="flex flex-col gap-1" data-testid="prompt">
          <details
            v-for="(part, at) in sections"
            :key="at"
            :open="at === 0"
            class="group rounded-xl bg-white/5 px-3 py-2"
          >
            <summary
              class="flex cursor-pointer list-none items-center gap-2 text-caption text-zinc-300 uppercase"
            >
              <span class="min-w-0 flex-1 truncate">{{ part.title }}</span>
              <span class="text-zinc-500"
                >{{ part.lines }} {{ part.lines === 1 ? 'line' : 'lines' }}</span
              >
              <AppIcon name="chevron" size="sm" class="transition group-open:rotate-180" />
            </summary>
            <pre
              class="m-0 mt-2 max-h-48 overflow-y-auto text-status break-words whitespace-pre-wrap text-zinc-300"
              >{{ part.body }}</pre>
          </details>
        </div>
      </template>

      <template v-else-if="tab === 'safety'">
        <div class="grid grid-cols-2 gap-2">
          <div
            v-for="item in [
              { label: 'Stated', value: step.riskStated },
              { label: 'Final', value: step.riskFinal },
            ]"
            :key="item.label"
            class="rounded-xl bg-white/5 p-3"
          >
            <span class="text-caption text-zinc-500 uppercase">{{ item.label }} risk</span>
            <p class="m-0 text-title">
              {{ riskLevel(item.value) >= 0 ? RISK[riskLevel(item.value)] : '—' }}
            </p>
            <div class="mt-2 flex gap-1" aria-hidden="true">
              <span
                v-for="n in 3"
                :key="n"
                class="h-1 flex-1 rounded-full"
                :class="
                  n - 1 <= riskLevel(item.value)
                    ? n === 3
                      ? 'bg-[#d71921]'
                      : 'bg-zinc-200'
                    : 'bg-white/10'
                "
              />
            </div>
          </div>
        </div>
        <p class="m-0 mt-3 flex items-center gap-2 text-caption text-zinc-500 uppercase">
          Gate
          <span class="rounded-full border border-white/25 px-2 text-zinc-200">{{
            step.gate ?? '—'
          }}</span>
        </p>
        <ul v-if="riskReasons.length" class="m-0 mt-3 flex list-none flex-col gap-1 p-0">
          <li
            v-for="reason in riskReasons"
            :key="reason"
            class="flex items-start gap-2 text-label text-zinc-300"
          >
            <span class="mt-1.5 size-1.5 shrink-0 rounded-full bg-zinc-400" />{{ reason }}
          </li>
        </ul>
        <template v-if="asked.length">
          <h3 class="mt-4 mb-1 text-caption text-zinc-500 uppercase">Questions and answers</h3>
          <ul class="m-0 flex list-none flex-col gap-1 p-0 text-status text-zinc-300">
            <li v-for="event in asked" :key="event.id" class="rounded-xl bg-white/5 px-3 py-2">
              <span class="text-caption text-zinc-500 uppercase">{{ event.kind }}</span>
              <br />{{ mask(event.dataJson) }}
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
          <div
            v-if="pickedCall.servedModel || pickedCall.swapMs"
            class="flex flex-wrap items-center gap-2 text-status"
            data-testid="call-model-info"
          >
            <span
              v-if="pickedCall.servedModel && pickedCall.servedModel !== pickedCall.model"
              class="rounded bg-amber-500/20 px-1.5 py-0.5 font-medium text-amber-300"
              data-testid="model-mismatch-warning"
            >
              ⚠ Served by {{ pickedCall.servedModel }} (requested {{ pickedCall.model }})
            </span>
            <span v-else-if="pickedCall.servedModel" class="text-zinc-400">
              Model: {{ pickedCall.servedModel }}
            </span>
            <span v-if="pickedCall.swapMs" class="text-zinc-400">
              Swap: {{ pickedCall.swapMs }} ms
            </span>
          </div>
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