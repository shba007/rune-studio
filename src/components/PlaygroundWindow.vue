<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onKeyStroke } from '@vueuse/core'
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch, type Ref } from 'vue'

import { useAgent } from '@/composables/useAgent'
import { useAnalysis } from '@/composables/useAnalysis'
import { useNativeBlur } from '@/composables/useNativeBlur'
import { useSectionHistory } from '@/composables/useSectionHistory'
import {
  usePlaygroundHistory,
  type ModelHistoryResult,
  type PlaygroundRun,
} from '@/composables/usePlaygroundHistory'
import { friendly } from '@/config/errors'
import { readImageAsPng } from '@/config/imageFile'
import {
  DEFAULT_PIPELINE,
  CREATE_ASPECTS,
  CREATE_RESOLUTIONS,
  MODEL_PIPELINES,
  PLAYGROUND_GROUPS,
  SCALE_OPTIONS,
  QUESTION_HINTS,
  RESOLUTION_OPTIONS,
  SOURCE_OPTIONS,
  createSize,
  filterFindings,
  findPipeline,
  idleRun,
  questionLimit,
  shapeQuestion,
  taskModelIds,
  speechTextBytes,
  unshapeQuestion,
  type CreateState,
  type EditState,
  type DepthState,
  type CaptionState,
  type ModelPipeline,
  type RunState,
  type UpscaleState,
  type SpeechState,
  type TranscribeState,
  type PlaygroundSource,
} from '@/config/playground'

import AppIcon from './AppIcon.vue'
import CaptureTargetPicker from './CaptureTargetPicker.vue'
import PlaygroundCreate from './PlaygroundCreate.vue'
import PlaygroundEdit from './PlaygroundEdit.vue'
import PlaygroundDepth from './PlaygroundDepth.vue'
import PlaygroundResult from './PlaygroundResult.vue'
import PlaygroundUpscale from './PlaygroundUpscale.vue'
import PlaygroundSpeech from './PlaygroundSpeech.vue'
import PlaygroundLoading from './PlaygroundLoading.vue'
import PlaygroundTranscript from './PlaygroundTranscript.vue'
import PlaygroundCaption from './PlaygroundCaption.vue'
import RuneButton from './RuneButton.vue'
import RuneChip from './RuneChip.vue'
import RuneChoice from './RuneChoice.vue'
import RuneNavItem from './RuneNavItem.vue'
import TitleBar from './TitleBar.vue'

interface TargetOption {
  target: { kind: string; id: number }
  label: string
  primary?: boolean
}

const agent = useAgent()
const analysis = useAnalysis()
const history = usePlaygroundHistory()
const windowRoot = useTemplateRef<HTMLElement>('windowRoot')
useNativeBlur(windowRoot, true)

const selectedId = ref(DEFAULT_PIPELINE)
const sidebarOpen = ref(true)
const nav = useSectionHistory(selectedId)
const menuItems = PLAYGROUND_GROUPS.flatMap((group) => group.pipelines).map((item) => ({
  id: item.id,
  label: item.label,
}))
const selected = computed(() => findPipeline(selectedId.value) ?? findPipeline(DEFAULT_PIPELINE)!)
const source = ref<PlaygroundSource>('screen')
const question = ref('')
const resolution = ref('')
const targets = ref<TargetOption[]>([])
const target = ref('')
const refreshing = ref(false)
const targetNotice = ref('')

const capable = computed(() =>
  (agent.backend.value?.models ?? []).filter(
    (model) => model.imageIn === 'yes' && model.toolCalls === 'yes',
  ),
)
const modelOptions = computed(() =>
  capable.value.length
    ? capable.value.map((model) => ({ value: model.id, label: model.id }))
    : [{ value: '', label: 'No vision model' }],
)
const model = ref('')
const activeModel = computed({
  get: () => model.value || agent.settings.value?.models.analysis || modelOptions.value[0]!.value,
  set: (value: string) => (model.value = value),
})
const activeResolution = computed({
  get: () => resolution.value || agent.settings.value?.modelImageResolution || '1280',
  set: (value: string) => (resolution.value = value),
})
const hints = computed(() => QUESTION_HINTS[selected.value.id] ?? [])

const modelPipeline = computed(() => MODEL_PIPELINES[selectedId.value])
const isModelRun = computed(() => !!modelPipeline.value)
const isCreate = computed(() => selectedId.value === 'create')
const isEdit = computed(() => selectedId.value === 'edit')
const isSpeech = computed(() => selectedId.value === 'tts')
const isCaption = computed(() => selectedId.value === 'caption')
const captionDetail = ref('concise')
const captionDetailOptions = [
  { value: 'concise', label: 'Concise' },
  { value: 'detailed', label: 'Detailed' },
]
const isTranscribe = computed(() => selectedId.value === 'transcribe')
const isTranslate = computed(() => selectedId.value === 'translate-speech')
const isAudioText = computed(() => isTranscribe.value || isTranslate.value)
const audioFile = ref<File | null>(null)
const audioNotice = ref('')
const audioInput = useTemplateRef<HTMLInputElement>('audioInput')
const speechText = ref('')
const speechBytes = computed(() => speechTextBytes(speechText.value))
const createResolution = ref('720')
const createAspect = ref('1:1')
const imageSize = computed(() => createSize(createResolution.value, createAspect.value))
const taskOptions = computed(() => {
  const pipeline = modelPipeline.value
  if (!pipeline) return []
  const ids = taskModelIds(
    agent.backend.value?.models ?? [],
    pipeline.task,
    pipeline.audioOperation,
  )
  return ids.length
    ? ids.map((id) => ({ value: id, label: id }))
    : [{ value: '', label: pipeline.none }]
})
const taskChoice = ref<Record<string, string>>({})
const activeTaskModel = computed({
  get: () => {
    const picked = taskChoice.value[selectedId.value]
    return taskOptions.value.some((item) => item.value === picked)
      ? picked!
      : (taskOptions.value[0]?.value ?? '')
  },
  set: (value: string) => (taskChoice.value = { ...taskChoice.value, [selectedId.value]: value }),
})
const depth = ref<DepthState>(idleRun())
const caption = ref<CaptionState>(idleRun())
const upscale = ref<UpscaleState>(idleRun())
const create = ref<CreateState>(idleRun())
const edit = ref<EditState>(idleRun())
const speech = ref<SpeechState>(idleRun())
const transcript = ref<TranscribeState>(idleRun())
const translation = ref<TranscribeState>(idleRun())
const modelRuns: Record<string, Ref<RunState<unknown>>> = {
  depth: depth as Ref<RunState<unknown>>,
  caption: caption as Ref<RunState<unknown>>,
  upscale: upscale as Ref<RunState<unknown>>,
  create: create as Ref<RunState<unknown>>,
  edit: edit as Ref<RunState<unknown>>,
  tts: speech as Ref<RunState<unknown>>,
  transcribe: transcript as Ref<RunState<unknown>>,
  'translate-speech': translation as Ref<RunState<unknown>>,
}
function shownState<T>(id: string, state: Ref<RunState<T>>) {
  return computed<RunState<T>>(() => {
    const viewed = history.viewed.value
    return viewed?.kind === 'model' && viewed.pipeline === id
      ? { status: 'done', result: viewed.result as T, error: '' }
      : state.value
  })
}
const shownDepth = shownState('depth', depth)
const shownCaption = shownState('caption', caption)
const shownUpscale = shownState('upscale', upscale)
const shownCreate = shownState('create', create)
const shownEdit = shownState('edit', edit)
const shownSpeech = shownState('tts', speech)
const shownTranscript = shownState('transcribe', transcript)
const shownTranslation = shownState('translate-speech', translation)
const startedAt = ref<Record<string, number>>({})
const activeRuns: Record<string, { runId?: number; cancelled: boolean }> = {}

function requestCancel(id: string, runId: number) {
  void invoke('playground_cancel_run', { runId }).catch(() => {
    const state = modelRuns[id]
    if (!activeRuns[id] && state?.value.status === 'idle')
      state.value = {
        status: 'error',
        result: null,
        error: 'Could not stop the request; Infer may still be processing it.',
      }
  })
}

function cancelModel(id: string) {
  const run = activeRuns[id]
  const state = modelRuns[id]
  if (!run || !state) return
  run.cancelled = true
  delete activeRuns[id]
  state.value = idleRun()
  if (run.runId !== undefined) requestCancel(id, run.runId)
}

onUnmounted(() => {
  for (const id of Object.keys(activeRuns)) cancelModel(id)
  history.clear()
})
const loading = computed(() =>
  isModelRun.value
    ? live.value && modelRuns[selectedId.value]?.value.status === 'running'
    : selected.value.ready && live.value && analysis.busy.value,
)
const loadingStartedAt = computed(
  () => startedAt.value[isModelRun.value ? selectedId.value : 'insight'] ?? Date.now(),
)
const loadingLabel = computed(
  () =>
    ({
      tts: 'Creating voice',
      create: 'Creating image',
      edit: 'Editing image',
      depth: 'Estimating depth',
      caption: 'Creating caption',
      upscale: 'Upscaling image',
      transcribe: 'Transcribing audio',
      'translate-speech': 'Translating speech',
    })[selectedId.value] ?? 'Analyzing image',
)
const scale = ref(2)
const headerModel = computed({
  get: () => (isModelRun.value ? activeTaskModel.value : activeModel.value),
  set: (value: string) => {
    if (isModelRun.value) activeTaskModel.value = value
    else activeModel.value = value
  },
})
const headerOptions = computed(() => (isModelRun.value ? taskOptions.value : modelOptions.value))

const options = computed(() => targets.value.filter((item) => item.target.kind === source.value))
const file = ref<{ name: string; data: string } | null>(null)
const fileNotice = ref('')
const dragging = ref(false)
const fileInput = useTemplateRef<HTMLInputElement>('fileInput')
const display = computed(() => {
  const viewed = history.viewed.value
  return filterFindings(
    selectedId.value,
    viewed?.kind === 'insight' ? viewed.snapshot : analysis.snapshot.value,
  )
})
const live = computed(() => !history.viewed.value)
const hasResult = computed(() => display.value.status !== 'idle')
const canRun = computed(
  () =>
    selected.value.ready &&
    (!isEdit.value || !!question.value.trim()) &&
    (isAudioText.value
      ? !!audioFile.value
      : isSpeech.value
        ? !!speechText.value.trim() && speechBytes.value <= 4096
        : isCreate.value
          ? !!question.value.trim()
          : source.value === 'file'
            ? !!file.value
            : !!target.value) &&
    (isModelRun.value
      ? !!activeTaskModel.value && modelRuns[selectedId.value]?.value.status !== 'running'
      : !analysis.busy.value),
)

function pickAudio(event: Event) {
  const input = event.target as HTMLInputElement
  const picked = input.files?.[0]
  input.value = ''
  if (!picked) return
  audioFile.value = null
  audioNotice.value = ''
  if (!picked.name.toLowerCase().endsWith('.wav')) audioNotice.value = 'Choose a WAV file.'
  else if (!picked.size || picked.size > 25 * 1024 * 1024)
    audioNotice.value = 'Choose a non-empty WAV file no larger than 25 MB.'
  else audioFile.value = picked
}

function readAudio(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () =>
      typeof reader.result === 'string'
        ? resolve(reader.result)
        : reject(new Error('Could not read the WAV file'))
    reader.onerror = () => reject(new Error('Could not read the WAV file'))
    reader.readAsDataURL(file)
  })
}

async function takeFile(picked: File | undefined) {
  fileNotice.value = ''
  if (!picked) return
  try {
    file.value = { name: picked.name, data: await readImageAsPng(picked) }
  } catch (error) {
    file.value = null
    fileNotice.value = error instanceof Error ? error.message : String(error)
  }
}
function onDrop(event: DragEvent) {
  dragging.value = false
  void takeFile(event.dataTransfer?.files?.[0])
}
function onPick(event: Event) {
  const input = event.target as HTMLInputElement
  void takeFile(input.files?.[0])
  input.value = ''
}

async function loadTargets() {
  if (refreshing.value) return
  refreshing.value = true
  targetNotice.value = ''
  try {
    const found = await invoke<unknown>('screen_insight_targets')
    targets.value = Array.isArray(found) ? (found as TargetOption[]) : []
    if (!targets.value.length)
      targetNotice.value = 'No capture targets are available. Check permissions and refresh.'
  } catch (error) {
    targets.value = []
    targetNotice.value = friendly(String(error))
  } finally {
    refreshing.value = false
  }
}
onMounted(loadTargets)

watch(
  [options, source],
  () => {
    if (options.value.some((item) => JSON.stringify(item.target) === target.value)) return
    const pick = options.value.find((item) => item.primary) ?? options.value[0]
    target.value = pick ? JSON.stringify(pick.target) : ''
  },
  { immediate: true },
)

let insightRun: {
  pipeline: string
  previousId: string
  id: string | null
  details: string[]
} | null = null
watch(
  () => analysis.snapshot.value,
  (snapshot) => {
    const run = insightRun
    if (!run || !snapshot.id || snapshot.id === run.previousId) return
    if (!run.id && ['capturing', 'asking', 'done'].includes(snapshot.status)) run.id = snapshot.id
    if (run.id !== snapshot.id) return
    if (snapshot.status === 'done') {
      history.record(run.pipeline, snapshot, run.details)
      insightRun = null
    } else if (snapshot.status === 'error' || snapshot.status === 'idle') insightRun = null
  },
)
watch(
  selectedId,
  (id) => {
    if (history.viewed.value && history.viewed.value.pipeline !== id) history.clearView()
  },
  { flush: 'sync' },
)

async function runModel(id: string, pipeline: ModelPipeline) {
  const option = options.value.find((item) => JSON.stringify(item.target) === target.value)
  const input =
    id === 'transcribe' || id === 'translate-speech'
      ? audioFile.value && { file: audioFile.value }
      : id === 'tts'
        ? { text: speechText.value }
        : id === 'create'
          ? { prompt: question.value, size: imageSize.value }
          : source.value === 'file'
            ? file.value && { image: file.value.data }
            : option && { target: option.target }
  const state = modelRuns[id]
  if (!input || !state) return
  history.clearView()
  const run: { runId?: number; cancelled: boolean } = { cancelled: false }
  activeRuns[id] = run
  startedAt.value = { ...startedAt.value, [id]: Date.now() }
  state.value = { status: 'running', result: null, error: '' }
  try {
    const extra =
      id === 'upscale'
        ? { scale: scale.value }
        : id === 'caption'
          ? { detail: captionDetail.value }
          : id === 'edit'
            ? { prompt: question.value }
            : {}
    const model = activeTaskModel.value
    const sourceLabel =
      source.value === 'file'
        ? (file.value?.name ?? 'Image file')
        : (option?.label ?? 'Selected target')
    const details =
      id === 'tts'
        ? [`Text: ${speechText.value}`]
        : id === 'create'
          ? [`Prompt: ${question.value}`, `Size: ${imageSize.value}`]
          : id === 'transcribe' || id === 'translate-speech'
            ? [`File: ${audioFile.value?.name ?? 'WAV file'}`]
            : [`Source: ${sourceLabel}`]
    if (id === 'caption') details.push(`Detail: ${captionDetail.value}`)
    if (id === 'upscale') details.push(`Scale: ${scale.value}×`)
    if (id === 'edit') details.push(`Prompt: ${question.value}`)
    details.unshift(`Model: ${model}`)
    const payload = 'file' in input ? { audio: await readAudio(input.file!) } : input
    if (run.cancelled) return
    run.runId = await invoke<number>('playground_start_run')
    if (run.cancelled) {
      requestCancel(id, run.runId)
      return
    }
    const result = await invoke<unknown>(pipeline.command, {
      runId: run.runId,
      input: { ...payload, model, ...extra },
    })
    if (!run.cancelled) {
      state.value = { status: 'done', result, error: '' }
      history.recordModel(id, result as ModelHistoryResult, details)
    }
  } catch (error) {
    if (!run.cancelled) state.value = { status: 'error', result: null, error: String(error) }
  } finally {
    if (!run.cancelled && run.runId !== undefined)
      void invoke('playground_cancel_run', { runId: run.runId }).catch(() => {})
    if (activeRuns[id] === run) delete activeRuns[id]
  }
}

function run() {
  if (!canRun.value) return
  if (modelPipeline.value) {
    void runModel(selectedId.value, modelPipeline.value)
    return
  }
  const shaped = shapeQuestion(selectedId.value, question.value)
  startedAt.value = { ...startedAt.value, insight: Date.now() }
  const overrides = { model: activeModel.value, resolution: activeResolution.value }
  const sourceLabel =
    source.value === 'file'
      ? (file.value?.name ?? 'Image file')
      : (options.value.find((item) => JSON.stringify(item.target) === target.value)?.label ??
        'Selected target')
  const pending = {
    pipeline: selectedId.value,
    previousId: analysis.snapshot.value.id,
    id: null,
    details: [
      `Source: ${sourceLabel}`,
      `Model: ${overrides.model}`,
      `Resolution: ${overrides.resolution}`,
      `Question: ${question.value || '(default)'}`,
    ],
  }
  if (source.value === 'file') {
    if (!file.value) return
    history.clearView()
    insightRun = pending
    void analysis.captureImage(shaped, file.value.data, overrides).then((ok) => {
      if (!ok && insightRun === pending) insightRun = null
    })
    return
  }
  const option = options.value.find((item) => JSON.stringify(item.target) === target.value)
  if (!option) return
  history.clearView()
  insightRun = pending
  void analysis.capture(shaped, option.target, overrides).then((ok) => {
    if (!ok && insightRun === pending) insightRun = null
  })
}

function openRun(id: string) {
  const run = history.runs.value.find((item) => item.id === id)
  if (!run) return
  selectedId.value = run.pipeline
  history.view(id)
  if (run.kind === 'insight') question.value = unshapeQuestion(run.pipeline, run.snapshot.question)
}

function runImage(run: PlaygroundRun): string | null {
  if (run.kind === 'insight') return run.snapshot.screenshot ?? null
  if ('image' in run.result) return run.result.image
  if ('original' in run.result) return run.result.original
  return null
}

onKeyStroke('Enter', (event) => {
  if (!event.ctrlKey) return
  event.preventDefault()
  run()
})
onKeyStroke('Escape', () => {
  if (isModelRun.value && loading.value) cancelModel(selectedId.value)
  else if (!isModelRun.value && analysis.busy.value) void analysis.cancel()
  else history.clearView()
})
</script>

<template>
  <main
    ref="windowRoot"
    class="flex h-screen flex-col overflow-hidden font-rune-reading text-rune-ink"
    data-testid="playground"
  >
    <TitleBar
      sidebar
      title="Playground"
      :collapsed="!sidebarOpen"
      :menu="menuItems"
      :can-back="nav.canBack.value"
      :can-forward="nav.canForward.value"
      @toggle="sidebarOpen = !sidebarOpen"
      @back="nav.back()"
      @forward="nav.forward()"
      @select="selectedId = $event"
    />
    <div class="flex min-h-0 flex-1">
      <nav
        v-show="sidebarOpen"
        class="glass flex w-64 shrink-0 flex-col bg-rune-sunken pb-4"
        aria-label="Pipelines"
        data-testid="playground-nav"
      >
        <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pt-2">
          <section v-for="group in PLAYGROUND_GROUPS" :key="group.id" class="flex flex-col gap-1">
            <h2 class="m-0 px-2 pb-1 text-rune-heading text-rune-text-3 uppercase">
              {{ group.label }}
            </h2>
            <RuneNavItem
              v-for="item in group.pipelines"
              :key="item.id"
              :selected="selectedId === item.id"
              :data-testid="`pipeline-${item.id}`"
              @click="selectedId = item.id"
            >
              <AppIcon :name="item.icon" />
              <span class="min-w-0 flex-1 truncate">{{ item.label }}</span>
              <span v-if="!item.ready" class="text-rune-caption text-rune-text-3">Later</span>
            </RuneNavItem>
          </section>
        </div>
      </nav>

      <section
        class="flex w-88 shrink-0 flex-col"
        aria-label="Settings"
        data-testid="playground-params"
      >
        <header data-tauri-drag-region class="flex h-12 shrink-0 items-center px-6">
          <RuneChoice
            v-model="headerModel"
            :label="modelPipeline?.modelLabel ?? 'Analysis model'"
            menu
            icon="model"
            :options="headerOptions"
          />
        </header>
        <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 pb-4">
          <header class="flex items-center gap-2">
            <AppIcon :name="selected.icon" />
            <h1 class="m-0 font-sans text-rune-title">{{ selected.label }}</h1>
          </header>
          <p class="m-0 text-rune-body text-rune-text-2">{{ selected.summary }}</p>
          <p
            v-if="modelPipeline?.audioOperation && !activeTaskModel"
            class="m-0 text-rune-caption text-rune-text-3"
          >
            Infer has not reported a model verified for this operation. Check the model's setup and
            verification in Infer.
          </p>

          <template v-if="selected.ready">
            <div v-if="isAudioText" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Audio file</span>
              <input
                ref="audioInput"
                type="file"
                accept=".wav,audio/wav"
                class="hidden"
                aria-label="WAV file"
                @change="pickAudio"
              />
              <RuneButton size="sm" class="self-start" @click="audioInput?.click()"
                >Choose WAV file</RuneButton
              >
              <span v-if="audioFile" class="break-all text-rune-caption text-rune-text-2">{{
                audioFile.name
              }}</span>
              <p class="m-0 text-rune-caption text-rune-text-3">
                16 kHz, mono, 16-bit PCM WAV, up to 25 MB.
              </p>
              <p v-if="isTranslate" class="m-0 text-rune-caption text-rune-text-3">
                Only models verified by Infer for speech translation are listed. Output language
                depends on the model.
              </p>
              <p v-if="audioNotice" role="alert" class="m-0 text-rune-caption text-error">
                {{ audioNotice }}
              </p>
            </div>
            <div v-if="!isCreate && !isSpeech && !isAudioText" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Source</span>
              <RuneChoice v-model="source" label="Source" :options="SOURCE_OPTIONS" />
              <div
                v-if="source === 'file'"
                class="flex flex-col gap-2"
                data-testid="source-target"
                @dragover.prevent="dragging = true"
                @dragleave="dragging = false"
                @drop.prevent="onDrop"
              >
                <input
                  ref="fileInput"
                  type="file"
                  accept="image/*"
                  class="hidden"
                  aria-label="Image file"
                  @change="onPick"
                />
                <div
                  class="grid aspect-square w-full place-items-center overflow-hidden rounded-rune-tile bg-rune-surface text-rune-body text-rune-text-2"
                  :class="dragging && 'outline-2 outline-accent'"
                >
                  <img
                    v-if="file"
                    :src="file.data"
                    :alt="file.name"
                    class="size-full object-contain"
                    data-testid="file-preview"
                  />
                  <span v-else class="flex items-center gap-2">
                    <AppIcon name="add" /> Drop an image or choose one
                  </span>
                </div>
                <div class="flex items-center gap-2">
                  <RuneButton size="sm" @click="fileInput?.click()">Choose image</RuneButton>
                  <span v-if="file" class="min-w-0 truncate text-rune-caption text-rune-text-2">
                    {{ file.name }}
                  </span>
                </div>
                <p v-if="fileNotice" role="alert" class="m-0 text-rune-caption text-error">
                  {{ fileNotice }}
                </p>
              </div>
              <template v-else>
                <CaptureTargetPicker
                  v-model="target"
                  :options="options"
                  :refreshing="refreshing"
                  @refresh="loadTargets"
                />
                <p v-if="targetNotice" role="alert" class="m-0 text-rune-caption text-error">
                  {{ targetNotice }}
                </p>
              </template>
            </div>

            <div v-if="isSpeech" class="flex flex-col gap-2">
              <label for="speech-text" class="text-rune-heading text-rune-text-3 uppercase"
                >Text</label
              >
              <textarea
                id="speech-text"
                v-model="speechText"
                rows="6"
                placeholder="Type the text to speak…"
                class="w-full resize-none rounded-rune-tile bg-rune-surface p-4 text-rune-body text-rune-ink outline-none placeholder:text-rune-text-3 focus-visible:outline-2 focus-visible:outline-accent"
              />
              <p
                :class="speechBytes > 4096 ? 'text-error' : 'text-rune-text-3'"
                class="m-0 text-rune-caption"
                aria-live="polite"
              >
                {{ speechBytes }} / 4096 bytes
              </p>
              <div class="flex flex-wrap gap-2">
                <RuneButton
                  v-for="hint in hints"
                  :key="hint"
                  size="sm"
                  class="normal-case!"
                  @click="speechText = hint"
                >
                  {{ hint }}
                </RuneButton>
              </div>
            </div>
            <div v-if="!isModelRun || isCreate || isEdit" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">{{
                isCreate || isEdit ? 'Prompt' : 'Question'
              }}</span>
              <textarea
                v-model="question"
                rows="4"
                :maxlength="questionLimit(selectedId)"
                aria-label="Question"
                :placeholder="
                  isEdit
                    ? 'Describe the change to make'
                    : isCreate
                      ? 'Describe the image to create'
                      : 'What do you want to know?'
                "
                class="w-full resize-none rounded-rune-tile bg-rune-surface p-4 text-rune-body text-rune-ink outline-none placeholder:text-rune-text-3 focus-visible:outline-2 focus-visible:outline-accent"
              />
              <div class="flex flex-wrap gap-2">
                <RuneButton
                  v-for="hint in hints"
                  :key="hint"
                  size="sm"
                  class="normal-case!"
                  @click="question = hint"
                >
                  {{ hint }}
                </RuneButton>
              </div>
            </div>

            <div v-if="!isModelRun" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Resolution</span>
              <RuneChoice
                v-model="activeResolution"
                label="Resolution"
                menu
                :options="RESOLUTION_OPTIONS"
              />
            </div>

            <div v-if="isCreate" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Resolution</span>
              <RuneChoice
                v-model="createResolution"
                label="Resolution"
                menu
                :options="CREATE_RESOLUTIONS"
              />
            </div>
            <div v-if="isCreate" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Aspect ratio</span>
              <RuneChoice
                v-model="createAspect"
                label="Aspect ratio"
                menu
                :options="CREATE_ASPECTS"
              />
            </div>

            <div v-if="selectedId === 'upscale'" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Scale</span>
              <RuneChoice v-model="scale" label="Scale" :options="SCALE_OPTIONS" />
            </div>
            <div v-if="isCaption" class="flex flex-col gap-2">
              <span class="text-rune-heading text-rune-text-3 uppercase">Detail</span>
              <RuneChoice v-model="captionDetail" label="Detail" :options="captionDetailOptions" />
            </div>
          </template>
          <RuneChip v-else class="self-start">Coming later</RuneChip>
        </div>
        <div v-if="selected.ready" class="shrink-0 px-6 pb-6">
          <RuneButton
            variant="primary"
            class="w-full"
            :disabled="!canRun"
            data-testid="playground-run"
            @click="run"
          >
            Run
          </RuneButton>
        </div>
      </section>

      <div class="relative flex min-w-0 flex-1 flex-col">
        <div
          v-if="history.viewed.value"
          class="flex max-h-28 shrink-0 items-start gap-4 overflow-auto bg-rune-sunken px-5 py-3"
          data-testid="history-details"
        >
          <div class="min-w-0 flex-1 text-rune-caption text-rune-text-2">
            <p class="m-0 font-semibold">
              {{ findPipeline(history.viewed.value.pipeline)?.label ?? 'Run' }} · Saved run
            </p>
            <p
              v-for="detail in history.viewed.value.details"
              :key="detail"
              class="m-0 whitespace-pre-wrap break-words"
            >
              {{ detail }}
            </p>
          </div>
          <RuneButton size="sm" @click="history.clearView()">Back to live</RuneButton>
        </div>
        <section
          class="m-0 flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-rune-surface bg-[radial-gradient(circle,rgb(255_255_255/0.22)_1px,transparent_1.5px)] bg-size-[24px_24px]"
          aria-label="Result"
          data-testid="playground-canvas"
        >
          <PlaygroundLoading
            v-if="loading"
            :started-at="loadingStartedAt"
            :label="loadingLabel"
            :cancellable="true"
            @cancel="isModelRun ? cancelModel(selectedId) : analysis.cancel()"
          />
          <PlaygroundDepth
            v-else-if="selectedId === 'depth' && shownDepth.status !== 'idle'"
            :state="shownDepth"
          />
          <PlaygroundCaption
            v-else-if="isCaption && shownCaption.status !== 'idle'"
            :state="shownCaption"
          />
          <PlaygroundUpscale
            v-else-if="selectedId === 'upscale' && shownUpscale.status !== 'idle'"
            :state="shownUpscale"
          />
          <PlaygroundCreate
            v-else-if="isCreate && shownCreate.status !== 'idle'"
            :state="shownCreate"
          />
          <PlaygroundEdit v-else-if="isEdit && shownEdit.status !== 'idle'" :state="shownEdit" />
          <PlaygroundSpeech
            v-else-if="isSpeech && shownSpeech.status !== 'idle'"
            :state="shownSpeech"
          />
          <PlaygroundTranscript
            v-else-if="isTranscribe && shownTranscript.status !== 'idle'"
            :state="shownTranscript"
          />
          <PlaygroundTranscript
            v-else-if="isTranslate && shownTranslation.status !== 'idle'"
            :state="shownTranslation"
          />
          <PlaygroundResult
            v-else-if="!isModelRun && hasResult && selected.ready"
            :snapshot="display"
            :live="live"
            @cancel="analysis.cancel()"
          />
          <div v-else class="flex max-w-80 flex-col items-center gap-3 text-center">
            <AppIcon :name="selected.icon" />
            <p class="m-0 text-rune-body text-rune-text-2">
              {{
                modelPipeline
                  ? modelPipeline.idle
                  : selected.ready
                    ? 'Results land here. Pick a source, ask a question and press Run.'
                    : `${selected.label} is planned for a later release.`
              }}
            </p>
          </div>
        </section>

        <p
          v-if="history.notice.value"
          role="status"
          class="m-0 shrink-0 bg-rune-sunken px-6 py-2 text-rune-caption text-rune-text-3"
        >
          {{ history.notice.value }}
        </p>
        <footer
          class="flex h-24 shrink-0 items-center gap-2 overflow-x-auto bg-rune-sunken px-6"
          aria-label="History"
          data-testid="playground-history"
        >
          <span v-if="!history.runs.value.length" class="text-rune-caption text-rune-text-3">
            This session's runs appear here.
          </span>
          <RuneButton
            v-for="item in history.runs.value"
            :key="item.id"
            variant="ghost"
            class="size-16 min-h-0 overflow-hidden rounded-rune-tile! p-0!"
            :aria-label="`Open ${findPipeline(item.pipeline)?.label ?? 'Playground'} run`"
            :title="findPipeline(item.pipeline)?.label ?? 'Playground run'"
            :class="history.viewed.value?.id === item.id && 'outline-2 outline-accent'"
            data-testid="history-run"
            @click="openRun(item.id)"
          >
            <img
              v-if="runImage(item)"
              :src="runImage(item)!"
              alt=""
              class="size-full object-cover"
            />
            <AppIcon v-else :name="findPipeline(item.pipeline)?.icon ?? 'image'" />
          </RuneButton>
        </footer>
      </div>
    </div>
  </main>
</template>