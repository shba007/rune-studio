<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { taskModelIds, type SpeechResult } from '@/config/playground'
import type { ModelCapabilities } from '@/types/protocol'
import AppIcon from './AppIcon.vue'
import PlaygroundFlowLayout from './PlaygroundFlowLayout.vue'
import PlaygroundFlowNode from './PlaygroundFlowNode.vue'
import PlaygroundFlowConnection from './PlaygroundFlowConnection.vue'
import PlaygroundImagePicker from './PlaygroundImagePicker.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'

const props = defineProps<{ models: ModelCapabilities[] }>()
const captionModels = computed(() => taskModelIds(props.models, 'image-to-text'))
const speechModels = computed(() => taskModelIds(props.models, 'text-to-audio', 'speechGeneration'))
const captionOptions = computed(() => [
  { value: '', label: 'Select a caption model', disabled: true },
  ...captionModels.value.map((id) => ({ value: id, label: id })),
])
const speechOptions = computed(() => [
  { value: '', label: 'Select a verified voice model', disabled: true },
  ...speechModels.value.map((id) => ({ value: id, label: id })),
])
const detailOptions = [
  { value: 'concise', label: 'Concise' },
  { value: 'detailed', label: 'Detailed' },
]
const captionModel = ref('')
const speechModel = ref('')
const detail = ref('concise')
const mode = ref<'app' | 'graph'>('app')
const file = ref<{ name: string; data: string } | null>(null)
const error = ref('')
watch(file, (image) => {
  if (image) error.value = ''
})
const caption = ref('')
const speech = ref<SpeechResult | null>(null)
const stage = ref<'idle' | 'caption' | 'speech' | 'done' | 'error'>('idle')
const failedStep = ref<'caption' | 'speech' | null>(null)
const runSettings = ref<{
  image: string
  captionModel: string
  speechModel: string
  detail: string
} | null>(null)
const started = ref(0)
const ready = ref(false)
let unlisten: UnlistenFn | undefined
let disposed = false
let current: { id?: number; cancelled: boolean } | undefined
const running = computed(() => stage.value === 'caption' || stage.value === 'speech')
const previousResult = computed(
  () =>
    !!runSettings.value &&
    !!(caption.value || speech.value) &&
    (runSettings.value.image !== file.value?.data ||
      runSettings.value.captionModel !== captionModel.value ||
      runSettings.value.speechModel !== speechModel.value ||
      runSettings.value.detail !== detail.value),
)
const captionStatus = computed(() =>
  stage.value === 'caption'
    ? 'Running'
    : caption.value
      ? 'Complete'
      : failedStep.value === 'caption'
        ? 'Failed'
        : 'Waiting',
)
const voiceStatus = computed(() =>
  stage.value === 'speech'
    ? 'Running'
    : speech.value
      ? 'Complete'
      : failedStep.value === 'speech'
        ? 'Failed'
        : 'Waiting',
)
const canRun = computed(
  () =>
    ready.value &&
    !running.value &&
    !!file.value &&
    captionModels.value.includes(captionModel.value) &&
    speechModels.value.includes(speechModel.value),
)

onMounted(async () => {
  try {
    unlisten = await listen<{ runId: number; stage: 'caption' | 'speech'; caption: string | null }>(
      'playground-flow-progress',
      ({ payload }) => {
        if (!current || current.cancelled || payload.runId !== current.id) return
        stage.value = payload.stage
        if (payload.caption) caption.value = payload.caption
      },
    )
    if (disposed) {
      unlisten()
      return
    }
    ready.value = true
  } catch {
    error.value = 'Could not listen for Flow progress. Reopen Playground and try again.'
  }
})

function cancel() {
  if (!current) return
  const run = current
  run.cancelled = true
  if (run.id !== undefined) void invoke('playground_cancel_run', { runId: run.id }).catch(() => {})
  current = undefined
  stage.value = 'idle'
  error.value = 'Flow cancelled.'
}

async function run() {
  if (!canRun.value || !file.value) return
  const active: { id?: number; cancelled: boolean } = { cancelled: false }
  current = active
  caption.value = ''
  speech.value = null
  error.value = ''
  failedStep.value = null
  runSettings.value = {
    image: file.value.data,
    captionModel: captionModel.value,
    speechModel: speechModel.value,
    detail: detail.value,
  }
  started.value = Date.now()
  stage.value = 'caption'
  try {
    active.id = await invoke<number>('playground_start_run')
    if (active.cancelled) {
      await invoke('playground_cancel_run', { runId: active.id })
      return
    }
    const result = await invoke<{
      caption: string
      speech: SpeechResult | null
      error: string | null
    }>('playground_image_voice_flow', {
      runId: active.id,
      input: {
        image: file.value.data,
        captionModel: captionModel.value,
        speechModel: speechModel.value,
        detail: detail.value,
      },
    })
    if (active.cancelled) return
    caption.value = result.caption
    speech.value = result.speech
    error.value = result.error ?? ''
    failedStep.value = result.error ? 'speech' : null
    stage.value = result.error ? 'error' : 'done'
  } catch (err) {
    if (!active.cancelled) {
      error.value = String(err)
      failedStep.value = caption.value ? 'speech' : 'caption'
      stage.value = 'error'
    }
  } finally {
    if (current === active) current = undefined
  }
}

onUnmounted(() => {
  disposed = true
  cancel()
  unlisten?.()
})
</script>

<template>
  <PlaygroundFlowLayout
    v-model="mode"
    data-testid="image-voice-flow"
    title="Image to Voice"
    settings-label="Flow settings"
    result-label="Flow result"
    graph-test-id="flow-graph"
    app-test-id="flow-app"
    :running="running"
    :started="started"
    :loading-label="stage === 'caption' ? 'Describing image' : 'Creating voice'"
    :error="error"
    :stale="previousResult"
    @cancel="cancel"
  >
    <template #settings>
      <div>
        <h1 class="m-0 text-rune-title">Image → Description → Voice</h1>
        <p class="text-rune-body text-rune-text-2">
          Describe an image, then turn the description into speech.
        </p>
      </div>
      <PlaygroundImagePicker
        id="flow-image"
        v-model="file"
        label="Flow image"
        :disabled="running"
      />
      <div class="flex flex-col items-start gap-2 text-rune-body">
        <span class="text-rune-heading text-rune-text-3 uppercase">Caption model</span>
        <RuneChoice
          v-model="captionModel"
          label="Flow caption model"
          :options="captionOptions"
          :disabled="running"
          model-icons
          menu
          floating
        />
      </div>
      <div class="flex flex-col items-start gap-2 text-rune-body">
        <span class="text-rune-heading text-rune-text-3 uppercase">Description detail</span>
        <RuneChoice
          v-model="detail"
          label="Description detail"
          :options="detailOptions"
          :disabled="running"
        />
      </div>
      <div class="flex flex-col items-start gap-2 text-rune-body">
        <span class="text-rune-heading text-rune-text-3 uppercase">Voice model</span>
        <RuneChoice
          v-model="speechModel"
          label="Flow voice model"
          :options="speechOptions"
          :disabled="running"
          model-icons
          menu
          floating
        />
      </div>
      <p
        v-if="!captionModels.length || !speechModels.length"
        class="m-0 text-rune-caption text-rune-text-3"
      >
        Infer must report a caption model and a verified speech-generation model.
      </p>
      <RuneButton variant="primary" :disabled="!canRun" data-testid="flow-run" @click="run"
        >Run Flow</RuneButton
      >
    </template>
    <template #graph>
      <PlaygroundFlowNode
        step="01"
        kind="Input"
        title="Image"
        :status="file ? 'Ready' : 'Not set'"
        icon="image"
        tone="sky"
        output
        class="max-w-[320px]"
        data-testid="flow-node-image"
      >
        <div
          class="mt-4 flex min-h-14 items-center gap-3 rounded-rune-tile border border-white/5 bg-black/20 p-3"
        >
          <img
            v-if="file"
            :src="file.data"
            :alt="file.name"
            class="size-12 shrink-0 rounded-rune-tile object-cover"
          />
          <span
            v-else
            class="grid size-12 shrink-0 place-items-center rounded-rune-tile bg-white/5 text-rune-text-3"
            ><AppIcon name="image"
          /></span>
          <p class="m-0 min-w-0 truncate text-rune-caption text-rune-text-2" :title="file?.name">
            {{ file?.name ?? 'Choose an image in the settings' }}
          </p>
        </div>
      </PlaygroundFlowNode>
      <PlaygroundFlowConnection label="Image" tone="sky" />
      <PlaygroundFlowNode
        step="02"
        kind="Operation"
        title="Caption"
        :status="captionStatus"
        icon="captions"
        tone="violet"
        :active="stage === 'caption'"
        input
        output
        class="max-w-[320px]"
        data-testid="flow-node-caption"
      >
        <div class="mt-4 rounded-rune-tile border border-white/5 bg-black/20 p-3">
          <p class="m-0 text-rune-caption text-rune-text-3">Model</p>
          <p class="mt-1 mb-0 break-all text-rune-body">
            {{ captionModel || 'Select a caption model' }}
          </p>
        </div>
        <p
          v-if="caption"
          class="mb-0 whitespace-pre-wrap break-words text-rune-caption text-rune-text-2"
        >
          {{ caption }}
        </p>
      </PlaygroundFlowNode>
      <PlaygroundFlowConnection label="Text" tone="violet" />
      <PlaygroundFlowNode
        step="03"
        kind="Operation"
        title="Create Voice"
        :status="voiceStatus"
        icon="speaker"
        tone="emerald"
        :active="stage === 'speech'"
        input
        class="max-w-[320px]"
        data-testid="flow-node-voice"
      >
        <div class="mt-4 rounded-rune-tile border border-white/5 bg-black/20 p-3">
          <p class="m-0 text-rune-caption text-rune-text-3">Model</p>
          <p class="mt-1 mb-0 break-all text-rune-body">
            {{ speechModel || 'Select a verified voice model' }}
          </p>
        </div>
        <audio v-if="speech" :src="speech.audio" controls class="mt-3 w-full" />
        <a
          v-if="speech"
          :href="speech.audio"
          download="rune-flow-voice.wav"
          class="mt-2 inline-block underline"
          >Download WAV</a
        >
      </PlaygroundFlowNode>
    </template>
    <template #app>
      <div v-if="caption" class="w-full max-w-2xl rounded-rune-tile bg-rune-sunken p-5">
        <p class="m-0 whitespace-pre-wrap break-words">{{ caption }}</p>
      </div>
      <div v-if="speech" class="w-full max-w-2xl">
        <audio :src="speech.audio" controls class="w-full" /><a
          :href="speech.audio"
          download="rune-flow-voice.wav"
          class="underline"
          >Download WAV</a
        >
      </div>
      <p v-if="stage === 'idle' && !error" class="text-rune-text-3">
        Choose an image and models to run this Flow.
      </p>
    </template>
  </PlaygroundFlowLayout>
</template>