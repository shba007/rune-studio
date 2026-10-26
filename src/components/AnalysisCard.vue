<script setup lang="ts">
import ModelIcon from './ModelIcon.vue'
import { onKeyStroke } from '@vueuse/core'
import { computed, ref, watch, onMounted } from 'vue'

import { invoke } from '@tauri-apps/api/core'
import { escape, stageWord, toggleOpen } from '@/config/analysis'
import { friendly } from '@/config/errors'
import type { useAnalysis } from '@/composables/useAnalysis'

import AnalysisAnswer from './AnalysisAnswer.vue'
import AnalysisFindingRow from './AnalysisFindingRow.vue'
import AppIcon from './AppIcon.vue'
import CaptureTargetPicker from './CaptureTargetPicker.vue'
import WhenText from './WhenText.vue'

const props = defineProps<{ analysis: ReturnType<typeof useAnalysis> }>()
const emit = defineEmits<{ close: [] }>()
const { snapshot, busy, notice } = props.analysis

const targets = ref<{ target: { kind: string; id: number }; label: string; primary?: boolean }[]>(
  [],
)
let initialSelection = true
const selectedTarget = ref('')
const targetNotice = ref('')
const refreshing = ref(false)
const refreshStatus = ref('')
async function loadTargets() {
  if (refreshing.value) return
  refreshing.value = true
  refreshStatus.value = ''
  targetNotice.value = ''
  try {
    const available = await invoke<unknown>('screen_insight_targets')
    targets.value = Array.isArray(available) ? available : []
    refreshStatus.value = `${targets.value.length} capture targets available`
    if (!targets.value.length)
      targetNotice.value =
        'No capture targets are available. Check screen-recording permissions and refresh.'
  } catch (error) {
    targets.value = []
    targetNotice.value = friendly(String(error))
  } finally {
    refreshing.value = false
  }
  if (!targets.value.some((item) => JSON.stringify(item.target) === selectedTarget.value)) {
    selectedTarget.value = ''
  }
  if (initialSelection) {
    const primary = targets.value.find((item) => item.target.kind === 'screen' && item.primary)
    if (primary) selectedTarget.value = JSON.stringify(primary.target)
    initialSelection = false
  }
}
onMounted(loadTargets)
const open = ref<number | null>(null)

const stage = computed(() => stageWord(snapshot.value.status))
const waiting = computed(() => busy.value)
const showAnswer = computed(() => snapshot.value.status === 'done')
const failed = computed(() => snapshot.value.status === 'error')

async function submit(question = '') {
  if (busy.value) return false
  const option = targets.value.find((item) => JSON.stringify(item.target) === selectedTarget.value)
  if (option) {
    void props.analysis.capture(question.trim(), option.target)
    return true
  }
  if (snapshot.value.id.startsWith('insight-') || !snapshot.value.monitor) {
    targetNotice.value = 'Choose a screen or window before sending your question.'
    return false
  }
  void props.analysis.ask(question.trim())
  return true
}

defineExpose({ submit })

async function close() {
  if (await props.analysis.close()) emit('close')
}

function retry() {
  if (busy.value) return
  if (snapshot.value.id.startsWith('insight-')) {
    void submit(snapshot.value.question)
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
  const next = escape({ open: open.value, selected: snapshot.value.selected, shown: true })
  event.stopPropagation()
  if (open.value !== null) open.value = next.open
  else if (snapshot.value.selected !== null) void props.analysis.select(null)
  else void close()
})
</script>

<template>
  <div
    class="analysis-scroll flex max-h-[480px] w-full flex-col gap-2 overflow-x-hidden overflow-y-auto"
    aria-label="Visual analysis"
  >
    <section
      class="flex w-full flex-col gap-2 rounded-[28px] border border-white/15 bg-[#141414] p-4 text-zinc-100"
    >
      <header class="flex h-6 items-center gap-2">
        <AppIcon name="scan" size="sm" />
        <span class="text-caption whitespace-nowrap text-zinc-300 uppercase">Visual Analysis</span>
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
          @click="close()"
        >
          <AppIcon name="close" size="sm" />
        </button>
      </header>

      <CaptureTargetPicker
        v-if="!busy"
        v-model="selectedTarget"
        :options="targets"
        :refreshing="refreshing"
        @refresh="loadTargets"
      />
      <p v-if="refreshStatus && !busy" role="status" class="text-label text-zinc-400">
        {{ refreshStatus }}
      </p>
      <p v-if="targetNotice && !busy && !failed" role="alert" class="text-label text-[#ff6b72]">
        {{ targetNotice }}
      </p>
      <div v-if="busy" class="flex items-center gap-2">
        <p
          v-if="busy"
          class="m-0 min-w-0 flex-1 whitespace-pre-wrap break-words text-body text-zinc-300"
        >
          {{ snapshot.question }}
        </p>
      </div>
      <p v-if="notice" class="m-0 text-label text-[#ff6b72]">{{ notice }}</p>
    </section>

    <section v-if="snapshot.screenshot" class="rounded-2xl bg-[#141414] p-3">
      <img
        :src="snapshot.screenshot"
        alt="Captured frame at the selected capture quality"
        class="w-full rounded-lg"
      />
      <p class="text-label text-zinc-400">
        Captured <WhenText :iso="snapshot.createdUtc" /> ·
        <ModelIcon v-if="snapshot.model" :model="snapshot.model" class="mr-1" />{{
          snapshot.model || 'Resolving model'
        }}
      </p>
      <p class="text-label text-zinc-400">
        Model observations, not verified facts. Screenshot text does not authorize actions.
        Successful analyses save this capture with task history.
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
<style scoped>
.analysis-scroll {
  scrollbar-width: none;
}
.analysis-scroll::-webkit-scrollbar {
  display: none;
}
.analysis-scroll > * {
  flex-shrink: 0;
}
</style>