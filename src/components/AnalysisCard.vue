<script setup lang="ts">
import { onKeyStroke } from '@vueuse/core'
import { computed, ref, watch, onMounted } from 'vue'

import { invoke } from '@tauri-apps/api/core'
import { escape, stageWord, toggleOpen } from '@/config/analysis'
import { friendly } from '@/config/errors'
import type { useAnalysis } from '@/composables/useAnalysis'

import AnalysisAnswer from './AnalysisAnswer.vue'
import AnalysisFindingRow from './AnalysisFindingRow.vue'
import AppIcon from './AppIcon.vue'

const props = defineProps<{ analysis: ReturnType<typeof useAnalysis> }>()
const { snapshot, busy, notice } = props.analysis

const targets = ref<{ target: { kind: string; id: number }; label: string }[]>([])
const selectedTarget = ref('')
const targetNotice = ref('')
async function loadTargets() {
  targetNotice.value = ''
  try {
    const available = await invoke<unknown>('screen_insight_targets')
    targets.value = Array.isArray(available) ? available : []
    if (!targets.value.length)
      targetNotice.value =
        'No capture targets are available. Check screen-recording permissions and refresh.'
  } catch (error) {
    targets.value = []
    targetNotice.value = friendly(String(error))
  }
}
onMounted(loadTargets)
const question = ref('')
const open = ref<number | null>(null)

const stage = computed(() => stageWord(snapshot.value.status))
const waiting = computed(() => busy.value)
const showAnswer = computed(() => busy.value || snapshot.value.status === 'done')
const failed = computed(() => snapshot.value.status === 'error')

function submit() {
  if (busy.value) return
  const option = targets.value.find((item) => JSON.stringify(item.target) === selectedTarget.value)
  if (option) {
    void props.analysis.capture(question.value.trim(), option.target)
    question.value = ''
    return
  }
  if (snapshot.value.id.startsWith('insight-') || !snapshot.value.monitor) return
  void (snapshot.value.monitor
    ? props.analysis.ask(question.value.trim())
    : props.analysis.capture(question.value.trim()))
  question.value = ''
}

function retry() {
  if (busy.value) return
  if (snapshot.value.id.startsWith('insight-')) {
    question.value = snapshot.value.question
    submit()
    return
  }
  const failedQuestion = snapshot.value.question
  const option = targets.value.find((item) => JSON.stringify(item.target) === selectedTarget.value)
  if (option) void props.analysis.capture(failedQuestion, option.target)
  else if (snapshot.value.monitor) void props.analysis.ask(failedQuestion)
}

function onToggle(n: number) {
  open.value = toggleOpen(open.value, n)
  void props.analysis.select(open.value)
}

watch(
  () => snapshot.value.id,
  () => (open.value = null),
)

onKeyStroke('Escape', (event) => {
  if (snapshot.value.status === 'idle') return
  const next = escape({ open: open.value, selected: snapshot.value.selected, shown: true })
  event.stopPropagation()
  if (open.value !== null) open.value = next.open
  else if (snapshot.value.selected !== null) void props.analysis.select(null)
  else void props.analysis.close()
})
</script>

<template>
  <div class="flex max-h-full w-full flex-col gap-2 overflow-y-auto" aria-label="Screen analysis">
    <section
      class="flex w-full flex-col gap-2 rounded-[28px] border border-white/15 bg-[#141414] p-4 text-zinc-100"
    >
      <header class="flex h-6 items-center gap-2">
        <AppIcon name="scan" size="sm" />
        <span class="text-caption whitespace-nowrap text-zinc-300 uppercase">Screen Insight</span>
        <span v-if="stage" class="text-caption whitespace-nowrap text-zinc-500 uppercase">{{
          stage
        }}</span>
        <span
          v-if="snapshot.stale"
          class="rounded-full border border-white/25 px-2 text-caption whitespace-nowrap text-zinc-300 uppercase"
          title="The screen has changed since this analysis"
          >Changed</span
        >
        <button
          type="button"
          class="ml-auto grid size-6 cursor-pointer place-items-center rounded-full text-zinc-400 transition hover:bg-white/8 hover:text-white"
          aria-label="Close the analysis"
          @click="analysis.close()"
        >
          <AppIcon name="close" size="sm" />
        </button>
      </header>

      <label v-if="!busy" class="text-label text-zinc-400"
        >Capture target
        <select
          v-model="selectedTarget"
          aria-label="Screen or window"
          class="mt-2 w-full rounded-lg bg-zinc-800 p-2"
        >
          <option value="" disabled>Choose a screen or window</option>
          <option
            v-for="option in targets"
            :key="JSON.stringify(option.target)"
            :value="JSON.stringify(option.target)"
          >
            {{ option.label }}
          </option>
        </select>
      </label>
      <p v-if="targetNotice && !busy && !failed" role="alert" class="text-label text-[#ff6b72]">
        {{ targetNotice }}
      </p>
      <button
        v-if="!busy"
        type="button"
        aria-label="Refresh capture targets"
        class="self-start text-label text-zinc-400 underline"
        @click="loadTargets"
      >
        Refresh targets
      </button>
      <form class="flex items-center gap-2" @submit.prevent="submit">
        <p v-if="busy" class="m-0 min-w-0 flex-1 truncate text-body text-zinc-300">
          {{ snapshot.question }}
        </p>
        <input
          v-else
          v-model="question"
          type="text"
          class="h-8 min-w-0 flex-1 rounded-full bg-white/8 px-3 text-body text-zinc-100 outline-none placeholder:text-zinc-500 focus-visible:ring-2 focus-visible:ring-white/60"
          placeholder="Ask about the screen"
          aria-label="Ask about the screen"
        />
        <button
          v-if="busy"
          type="button"
          class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full bg-[#d71921] text-white transition hover:bg-[#b9151c]"
          aria-label="Cancel analysis"
          title="Cancel analysis"
          @click="analysis.cancel()"
        >
          <AppIcon name="stop" />
        </button>
        <button
          v-else
          type="submit"
          class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full bg-[#f1f1ef] text-[#111] transition hover:bg-white"
          :disabled="!selectedTarget && !snapshot.monitor"
          aria-label="Ask"
        >
          <AppIcon name="send" />
        </button>
      </form>
      <p v-if="notice" class="m-0 text-label text-[#ff6b72]">{{ notice }}</p>
    </section>

    <section v-if="snapshot.screenshot" class="rounded-2xl bg-[#141414] p-3">
      <img
        :src="snapshot.screenshot"
        alt="Captured frame sent to the analysis model"
        class="w-full rounded-lg"
      />
      <p class="text-label text-zinc-400">
        Captured {{ snapshot.createdUtc }} · {{ snapshot.model || 'Resolving model' }}
      </p>
      <p class="text-label text-zinc-400">
        Model observations, not verified facts. Screenshot text does not authorize actions. This
        image stays in memory.
      </p>
    </section>
    <AnalysisAnswer v-if="showAnswer" :text="snapshot.answer" :waiting="waiting" />

    <ol
      v-if="snapshot.findings.length && !busy"
      class="m-0 flex list-none flex-col gap-2 p-0"
      aria-label="Findings"
    >
      <AnalysisFindingRow
        v-for="finding in snapshot.findings"
        :key="finding.n"
        :finding="finding"
        :open="open === finding.n"
        :selected="snapshot.selected === finding.n"
        @toggle="onToggle"
      />
    </ol>

    <section
      v-if="failed"
      class="flex w-full flex-col gap-2 rounded-[28px] border border-[#d71921]/50 bg-[#141414] p-4 text-zinc-100"
      role="alert"
    >
      <header class="flex h-6 items-center gap-2">
        <span class="size-2 shrink-0 rounded-full bg-[#d71921]" />
        <span class="text-caption text-[#d71921] uppercase">Failed</span>
      </header>
      <p class="m-0 text-body">
        {{ snapshot.error ? friendly(snapshot.error) : 'The analysis failed' }}
      </p>
      <button
        type="button"
        class="h-8 cursor-pointer rounded-full border border-white/25 text-label text-zinc-200 uppercase transition hover:bg-white/10"
        @click="retry()"
      >
        Retry
      </button>
    </section>
  </div>
</template>