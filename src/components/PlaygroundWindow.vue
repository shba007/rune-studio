<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onKeyStroke } from '@vueuse/core'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import { useAnalysis } from '@/composables/useAnalysis'
import { useNativeBlur } from '@/composables/useNativeBlur'
import { usePlaygroundHistory } from '@/composables/usePlaygroundHistory'
import { friendly } from '@/config/errors'
import { readImageAsPng } from '@/config/imageFile'
import {
  DEFAULT_PIPELINE,
  DEPTH_PIPELINE,
  IDLE_DEPTH,
  PLAYGROUND_GROUPS,
  QUESTION_HINTS,
  RESOLUTION_OPTIONS,
  SOURCE_OPTIONS,
  depthModelIds,
  filterFindings,
  findPipeline,
  questionLimit,
  shapeQuestion,
  unshapeQuestion,
  type DepthResult,
  type DepthState,
  type PlaygroundSource,
} from '@/config/playground'

import AppIcon from './AppIcon.vue'
import CaptureTargetPicker from './CaptureTargetPicker.vue'
import PlaygroundDepth from './PlaygroundDepth.vue'
import PlaygroundResult from './PlaygroundResult.vue'
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

const isDepth = computed(() => selectedId.value === DEPTH_PIPELINE)
const depthOptions = computed(() => {
  const ids = depthModelIds(agent.backend.value?.models ?? [])
  return ids.length
    ? ids.map((id) => ({ value: id, label: id }))
    : [{ value: '', label: 'No depth model' }]
})
const depthModel = ref('')
const activeDepthModel = computed({
  get: () =>
    depthOptions.value.some((item) => item.value === depthModel.value)
      ? depthModel.value
      : depthOptions.value[0]!.value,
  set: (value: string) => (depthModel.value = value),
})
const depth = ref<DepthState>(IDLE_DEPTH)
const headerModel = computed({
  get: () => (isDepth.value ? activeDepthModel.value : activeModel.value),
  set: (value: string) => {
    if (isDepth.value) activeDepthModel.value = value
    else activeModel.value = value
  },
})
const headerOptions = computed(() => (isDepth.value ? depthOptions.value : modelOptions.value))

const options = computed(() => targets.value.filter((item) => item.target.kind === source.value))
const file = ref<{ name: string; data: string } | null>(null)
const fileNotice = ref('')
const dragging = ref(false)
const fileInput = useTemplateRef<HTMLInputElement>('fileInput')
const display = computed(() =>
  filterFindings(selectedId.value, history.viewed.value?.snapshot ?? analysis.snapshot.value),
)
const live = computed(() => !history.viewed.value)
const hasResult = computed(() => display.value.status !== 'idle')
const canRun = computed(
  () =>
    selected.value.ready &&
    (source.value === 'file' ? !!file.value : !!target.value) &&
    (isDepth.value
      ? !!activeDepthModel.value && depth.value.status !== 'running'
      : !analysis.busy.value),
)

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

watch(
  () => analysis.snapshot.value,
  (snapshot) => history.record(selectedId.value, snapshot),
)

async function runDepth() {
  const option = options.value.find((item) => JSON.stringify(item.target) === target.value)
  const input =
    source.value === 'file'
      ? file.value && { image: file.value.data }
      : option && { target: option.target }
  if (!input) return
  depth.value = { status: 'running', result: null, error: '' }
  try {
    const result = await invoke<DepthResult>('playground_depth', {
      input: { ...input, model: activeDepthModel.value },
    })
    depth.value = { status: 'done', result, error: '' }
  } catch (error) {
    depth.value = { status: 'error', result: null, error: String(error) }
  }
}

function run() {
  if (!canRun.value) return
  if (isDepth.value) {
    void runDepth()
    return
  }
  const shaped = shapeQuestion(selectedId.value, question.value)
  const overrides = { model: activeModel.value, resolution: activeResolution.value }
  if (source.value === 'file') {
    if (!file.value) return
    history.clearView()
    void analysis.captureImage(shaped, file.value.data, overrides)
    return
  }
  const option = options.value.find((item) => JSON.stringify(item.target) === target.value)
  if (!option) return
  history.clearView()
  void analysis.capture(shaped, option.target, overrides)
}

function openRun(id: string) {
  const run = history.runs.value.find((item) => item.id === id)
  if (!run) return
  history.view(id)
  selectedId.value = run.pipeline
  question.value = unshapeQuestion(run.pipeline, run.snapshot.question)
}

onKeyStroke('Enter', (event) => {
  if (!event.ctrlKey) return
  event.preventDefault()
  run()
})
onKeyStroke('Escape', () => {
  if (analysis.busy.value) void analysis.cancel()
  else history.clearView()
})
</script>

<template>
  <main
    ref="windowRoot"
    class="flex h-screen overflow-hidden font-rune-reading text-rune-ink"
    data-testid="playground"
  >
    <nav
      class="glass flex w-72 shrink-0 flex-col bg-rune-sunken pb-4"
      aria-label="Pipelines"
      data-testid="playground-nav"
    >
      <div data-tauri-drag-region class="flex h-10 shrink-0 items-center gap-2 px-4 select-none">
        <AppIcon name="logo" />
        <span class="text-rune-heading text-rune-text-2 uppercase">Playground</span>
      </div>
      <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pt-4">
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

    <div class="flex min-w-0 flex-1 flex-col">
      <TitleBar bare title="Playground" />
      <header class="flex h-12 shrink-0 items-center gap-4 px-6">
        <RuneChoice
          v-model="headerModel"
          :label="isDepth ? 'Depth model' : 'Analysis model'"
          menu
          :options="headerOptions"
        />
      </header>

      <div class="flex min-h-0 flex-1">
        <section
          class="flex w-88 shrink-0 flex-col"
          aria-label="Settings"
          data-testid="playground-params"
        >
          <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 pb-4">
            <header class="flex items-center gap-2">
              <AppIcon :name="selected.icon" />
              <h1 class="m-0 font-sans text-rune-title">{{ selected.label }}</h1>
            </header>
            <p class="m-0 text-rune-body text-rune-text-2">{{ selected.summary }}</p>

            <template v-if="selected.ready">
              <div class="flex flex-col gap-2">
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
                    class="grid h-24 place-items-center overflow-hidden rounded-rune-tile bg-rune-surface text-rune-body text-rune-text-2"
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

              <div v-if="!isDepth" class="flex flex-col gap-2">
                <span class="text-rune-heading text-rune-text-3 uppercase">Question</span>
                <textarea
                  v-model="question"
                  rows="4"
                  :maxlength="questionLimit(selectedId)"
                  aria-label="Question"
                  placeholder="What do you want to know?"
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

              <div v-if="!isDepth" class="flex flex-col gap-2">
                <span class="text-rune-heading text-rune-text-3 uppercase">Resolution</span>
                <RuneChoice
                  v-model="activeResolution"
                  label="Resolution"
                  menu
                  :options="RESOLUTION_OPTIONS"
                />
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

        <section
          class="m-0 mr-6 mb-4 flex min-w-0 flex-1 items-center justify-center overflow-hidden rounded-rune-tile bg-rune-surface"
          aria-label="Result"
          data-testid="playground-canvas"
        >
          <PlaygroundDepth v-if="isDepth && depth.status !== 'idle'" :state="depth" />
          <PlaygroundResult
            v-else-if="!isDepth && hasResult && selected.ready"
            :snapshot="display"
            :live="live"
            @cancel="analysis.cancel()"
          />
          <div v-else class="flex max-w-80 flex-col items-center gap-3 text-center">
            <AppIcon :name="selected.icon" />
            <p class="m-0 text-rune-body text-rune-text-2">
              {{
                isDepth
                  ? 'Pick a source and a depth model, then press Run to see the depth map.'
                  : selected.ready
                    ? 'Results land here. Pick a source, ask a question and press Run.'
                    : `${selected.label} is planned for a later release.`
              }}
            </p>
          </div>
        </section>
      </div>

      <footer
        class="flex h-24 shrink-0 items-center gap-2 overflow-x-auto px-6 pb-4"
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
          :aria-label="`Open run: ${item.snapshot.question || 'Describe the screen'}`"
          :class="history.viewed.value?.id === item.id && 'outline-2 outline-accent'"
          data-testid="history-run"
          @click="openRun(item.id)"
        >
          <img
            v-if="item.snapshot.screenshot"
            :src="item.snapshot.screenshot"
            alt=""
            class="size-full object-cover"
          />
          <AppIcon v-else name="image" />
        </RuneButton>
      </footer>
    </div>
  </main>
</template>