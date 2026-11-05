<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { useDebounceFn } from '@vueuse/core'
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import type { ModelCapabilities } from '@/types/protocol'
import { inspectRawFolder, readRawFile } from '@/config/rawFiles'
import PlaygroundFlowLayout from './PlaygroundFlowLayout.vue'
import PlaygroundFlowNode from './PlaygroundFlowNode.vue'
import PlaygroundFlowConnection from './PlaygroundFlowConnection.vue'
import ModelIcon from './ModelIcon.vue'
import RuneButton from './RuneButton.vue'

const props = defineProps<{ models: ModelCapabilities[] }>()
const mode = ref<'app' | 'graph'>('app')
const stage = ref<'import' | 'select' | 'create'>('import')
const stages = [
  { id: 'import', label: 'Import photos', number: '01' },
  { id: 'select', label: 'Choose views', number: '02' },
  { id: 'create', label: 'Create & review', number: '03' },
] as const
const models = computed(() =>
  props.models.filter(
    (m) => m.imageIn === 'yes' && m.toolCalls === 'yes' && m.locality === 'local',
  ),
)
const model = ref('')
const options = computed(() => [
  { value: '', label: 'Select a vision model' },
  ...models.value.map((m) => ({ value: m.id, label: m.id })),
])
const matteOptions = computed(() => [
  { value: '', label: 'Select a cutout model', disabled: true },
  ...props.models
    .filter(
      (m) =>
        m.locality === 'local' &&
        ((m.tasks?.includes('image-to-matting') && m.portraitMattingVerified) ||
          (m.tasks?.includes('text-image-to-image') &&
            m.rawCapabilities?.some((c) => c.toLowerCase() === 'transparency (rgba)'))),
    )
    .map((m) => ({ value: m.id, label: m.id })),
])
const matteModel = ref('')
const visionReady = computed(() => models.value.some((item) => item.id === model.value))
const editReady = computed(() =>
  matteOptions.value.some((item) => item.value && item.value === matteModel.value),
)
const views = [
  'front',
  'left three-quarter',
  'right three-quarter',
  'back',
  'waist-detail',
] as const
type View = (typeof views)[number]
type Cutout = {
  original: string
  cutout: string
  composite: string
  model: string
  elapsedMs: number
  approved: boolean
  attempt: number
  review?: string
  reviewModel?: string
}
function reviewDecision(review?: string) {
  const match = review?.trim().match(/^DECISION:\s*(ACCEPT|RETRY|REVIEW)\b/i)
  return match?.[1]?.toLowerCase() ?? 'unknown'
}
type DevelopedRaw = { path: string; width: number; height: number }
const cutouts = ref<Partial<Record<View, Cutout>>>({})
const previousCutouts = ref<Partial<Record<View, Cutout>>>({})
const attempts = ref<Partial<Record<View, number>>>({})
const developed = ref<Partial<Record<View, DevelopedRaw>>>({})
const developing = ref(false)
const exported = ref<Partial<Record<View, DevelopedRaw>>>({})
const cutoutView = ref<View>('front')
const currentCutout = computed(() => cutouts.value[cutoutView.value])
const automaticLook = ref(true)
const automaticLookReady = ref(false)
watch(automaticLook, (enabled) => {
  if (enabled) automaticLookReady.value = false
})
const finishSettings = ref({
  crop: 'original',
  autoCenter: true,
  scale: 100,
  horizontal: 0,
  vertical: 0,
  exposure: 0,
  warmth: 0,
  backgroundLeft: 160,
  backgroundRight: 135,
})
const finishSliders: {
  key:
    | 'scale'
    | 'horizontal'
    | 'vertical'
    | 'exposure'
    | 'warmth'
    | 'backgroundLeft'
    | 'backgroundRight'
  label: string
  min: number
  max: number
  step: number
}[] = [
  { key: 'scale', label: 'Subject scale (%)', min: 50, max: 150, step: 1 },
  { key: 'horizontal', label: 'Move horizontally', min: -50, max: 50, step: 1 },
  { key: 'vertical', label: 'Move vertically', min: -50, max: 50, step: 1 },
  { key: 'exposure', label: 'Exposure', min: -100, max: 100, step: 5 },
  { key: 'warmth', label: 'Warmth', min: -100, max: 100, step: 5 },
  { key: 'backgroundLeft', label: 'Background left', min: 0, max: 255, step: 1 },
  { key: 'backgroundRight', label: 'Background right', min: 0, max: 255, step: 1 },
]
const finish = ref<{ original: string; composite: string } | null>(null)
const finishPending = ref(false)
const finishError = ref('')
let finishRevision = 0
let disposed = false
const renderFinish = useDebounceFn(
  async (
    revision: number,
    input: { original: string; cutout: string; settings: typeof finishSettings.value },
  ) => {
    try {
      const result = await invoke<{ original: string; composite: string }>(
        'portfolio_finish_preview',
        { input },
      )
      if (!disposed && revision === finishRevision) finish.value = result
    } catch (e) {
      if (!disposed && revision === finishRevision) finishError.value = String(e)
    } finally {
      if (!disposed && revision === finishRevision) finishPending.value = false
    }
  },
  150,
)
watch(
  finishSettings,
  () => {
    exported.value = {}
    for (const candidate of [
      ...Object.values(cutouts.value),
      ...Object.values(previousCutouts.value),
    ]) {
      if (candidate) {
        candidate.approved = false
        delete candidate.review
        delete candidate.reviewModel
      }
    }
  },
  { deep: true },
)
watch(
  () =>
    [
      cutoutView.value,
      currentCutout.value?.attempt,
      currentCutout.value?.original,
      currentCutout.value?.cutout,
      { ...finishSettings.value },
    ] as const,
  () => {
    const revision = ++finishRevision
    finish.value = null
    finishError.value = ''
    const candidate = currentCutout.value
    if (!candidate) {
      finishPending.value = false
      return
    }
    finishPending.value = true
    void renderFinish(revision, {
      original: candidate.original,
      cutout: candidate.cutout,
      settings: { ...finishSettings.value },
    })
  },
  { immediate: true },
)
type Candidate = {
  file: File
  path: string
  blocked: boolean
  preview: string
  developedPreview?: string
  suggestion: string
  error: string
}
const candidates = ref<Candidate[]>([])
const selection = ref<Record<View, string>>({
  front: '',
  'left three-quarter': '',
  'right three-quarter': '',
  back: '',
  'waist-detail': '',
})
const running = ref(false)
const started = ref(0)
const progress = ref('')
const error = ref('')
const approved = ref(false)
let currentRun: number | undefined
let cancelled = false
const ready = computed(() => candidates.value.filter((c) => !c.blocked))
const blocked = computed(() => candidates.value.filter((c) => c.blocked))
const previewed = computed(() => ready.value.filter((c) => c.preview))
const activeView = ref<View>('front')
const selectedCount = computed(() => Object.values(selection.value).filter(Boolean).length)

function assignPhoto(path: string) {
  if (running.value || !previewed.value.some((item) => item.path === path)) return
  if (selection.value[activeView.value] === path) return
  selection.value[activeView.value] = path
  selectionChanged(activeView.value)
}

function resetSelection() {
  selection.value = {
    front: '',
    'left three-quarter': '',
    'right three-quarter': '',
    back: '',
    'waist-detail': '',
  }
  approved.value = false
  cutouts.value = {}
  previousCutouts.value = {}
  attempts.value = {}
  developed.value = {}
  exported.value = {}
  automaticLookReady.value = false
}
function selectionChanged(view: View) {
  approved.value = false
  automaticLookReady.value = false
  delete cutouts.value[view]
  delete previousCutouts.value[view]
  delete attempts.value[view]
  delete developed.value[view]
  delete exported.value[view]
}
function chooseFolder(event: Event) {
  const input = event.target as HTMLInputElement
  try {
    candidates.value = inspectRawFolder(Array.from(input.files ?? [])).map((item) => ({
      ...item,
      preview: '',
      suggestion: '',
      error: '',
    }))
    resetSelection()
    stage.value = 'import'
    error.value = ''
  } catch (e) {
    candidates.value = []
    resetSelection()
    stage.value = 'import'
    error.value = String(e)
  } finally {
    input.value = ''
  }
}
function cancel() {
  cancelled = true
  if (currentRun !== undefined)
    void invoke('playground_cancel_run', { runId: currentRun }).catch(() => {})
}
onUnmounted(() => {
  disposed = true
  finishRevision++
  cancel()
})
async function run(useSelections = false) {
  if (running.value || !ready.value.length) return
  if (!visionReady.value) {
    error.value = 'Choose a local vision model before running.'
    return
  }
  if (!editReady.value) {
    error.value = 'Choose a cutout model before running.'
    return
  }
  cancelled = false
  running.value = true
  started.value = Date.now()
  error.value = ''
  const visionModel = model.value
  const editModel = matteModel.value
  let look = { ...finishSettings.value }
  async function call<T>(command: string, input: unknown): Promise<T> {
    currentRun = await invoke<number>('playground_start_run')
    if (cancelled) {
      await invoke('playground_cancel_run', { runId: currentRun })
      throw new Error('Run cancelled.')
    }
    try {
      return await invoke<T>(command, { runId: currentRun, input })
    } finally {
      currentRun = undefined
    }
  }
  for (const candidate of useSelections ? [] : ready.value) {
    if (cancelled) break
    candidate.error = ''
    candidate.suggestion = ''
    progress.value = `Preparing ${candidate.file.name}`
    try {
      if (!candidate.preview)
        candidate.preview = await invoke<string>('portfolio_raw_preview', {
          raw: await readRawFile(candidate.file),
        })
      if (cancelled) break
      if (visionModel) {
        progress.value = `Classifying ${candidate.file.name}`
        const answer = await call<string>('portfolio_classify', {
          image: candidate.preview,
          model: visionModel,
        })
        const normalized = answer.trim().toLowerCase()
        const view = views.find(
          (v) =>
            normalized === v ||
            normalized.startsWith(`${v}\n`) ||
            normalized.startsWith(`${v}.`) ||
            normalized.startsWith(`${v}:`),
        )
        candidate.suggestion = view ?? 'uncertain'
        if (view && !selection.value[view]) selection.value[view] = candidate.path
      }
    } catch (e) {
      if (!cancelled) candidate.error = String(e)
    }
  }
  const selected = views.filter((view) => selection.value[view])
  if (!cancelled && selected.length) {
    approved.value = true
    cutoutView.value = selected[0]!
    stage.value = 'create'
    for (const view of selected) {
      if (cancelled) break
      const source = ready.value.find((item) => item.path === selection.value[view])
      if (source && useSelections) source.error = ''
      if (!source?.preview || cutouts.value[view]) continue
      try {
        if (!source.developedPreview) {
          progress.value = `Developing ${view} preview from RAW`
          source.developedPreview = await call<string>('portfolio_developed_preview', {
            raw: await readRawFile(source.file),
          })
        }
        if (cancelled) break
        source.preview = source.developedPreview
      } catch (e) {
        if (!cancelled) source.error = `Could not develop preview: ${String(e)}`
        continue
      }
      for (let attempt = 0; attempt < 2 && !cancelled; attempt++) {
        progress.value = `Creating ${view} preview`
        try {
          attempts.value[view] = attempt + 1
          const result = await call<Omit<Cutout, 'approved' | 'attempt'>>(
            'playground_extract_subject',
            { image: source.preview, model: editModel, retry: attempt > 0 },
          )
          if (!cancelled) cutouts.value[view] = { ...result, approved: false, attempt: attempt + 1 }
          break
        } catch (e) {
          if (!cancelled && attempt === 1) source.error = `Cutout failed: ${String(e)}`
        }
      }
    }
    if (!cancelled && automaticLook.value && !automaticLookReady.value) {
      const samples = selected
        .map((view) => cutouts.value[view])
        .filter((item): item is Cutout => !!item)
        .map((item) => ({ original: item.original, cutout: item.cutout }))
      if (samples.length) {
        try {
          progress.value = 'Estimating a consistent shoot look'
          const suggestion = await invoke<{ exposure: number; warmth: number }>(
            'portfolio_suggest_look',
            { samples },
          )
          if (cancelled) {
            error.value = 'Run cancelled.'
            running.value = false
            currentRun = undefined
            return
          }
          finishSettings.value.exposure = suggestion.exposure
          finishSettings.value.warmth = suggestion.warmth
          automaticLookReady.value = true
          await nextTick()
          look = { ...finishSettings.value }
        } catch (e) {
          if (!cancelled)
            error.value = `Could not estimate the shoot look: ${String(e)}. Turn off automatic look to use manual adjustments.`
          running.value = false
          currentRun = undefined
          return
        }
      }
    }
    async function reviewView(view: View) {
      const candidate = cutouts.value[view]
      const source = ready.value.find((item) => item.path === selection.value[view])
      if (!candidate || !source || candidate.review || cancelled) return
      progress.value = `Reviewing ${view} preview`
      try {
        const rendered = await invoke<{ original: string; composite: string }>(
          'portfolio_finish_preview',
          { input: { original: candidate.original, cutout: candidate.cutout, settings: look } },
        )
        if (cancelled) return
        const review = await call<string>('portfolio_review', {
          original: rendered.original,
          candidate: rendered.composite,
          model: visionModel,
        })
        if (!cancelled) {
          candidate.review = review
          candidate.reviewModel = visionModel
        }
      } catch (e) {
        if (!cancelled) source.error = `Vision review failed: ${String(e)}`
      }
    }
    for (const view of selected) {
      if (cancelled) break
      await reviewView(view)
    }
    const deterministicMatte = props.models.some(
      (item) => item.id === editModel && item.portraitMattingVerified,
    )
    const retries = selected.filter(
      (view) =>
        !deterministicMatte &&
        reviewDecision(cutouts.value[view]?.review) === 'retry' &&
        (attempts.value[view] ?? 0) < 2,
    )
    for (const view of retries) {
      if (cancelled) break
      const source = ready.value.find((item) => item.path === selection.value[view])
      const previous = cutouts.value[view]
      if (!source?.preview || !previous) continue
      progress.value = `Retrying ${view} from original`
      try {
        attempts.value[view] = 2
        const result = await call<Omit<Cutout, 'approved' | 'attempt'>>(
          'playground_extract_subject',
          { image: source.preview, model: editModel, retry: true },
        )
        if (!cancelled) {
          previousCutouts.value[view] = previous
          cutouts.value[view] = { ...result, approved: false, attempt: 2 }
        }
      } catch (e) {
        if (!cancelled) source.error = `Review retry failed: ${String(e)}`
      }
    }
    for (const view of retries) {
      if (cancelled) break
      await reviewView(view)
    }
    const failures = selected.filter((view) => {
      const source = ready.value.find((item) => item.path === selection.value[view])
      const decision = reviewDecision(cutouts.value[view]?.review)
      if (source && !source.error && decision !== 'accept')
        source.error =
          decision === 'retry'
            ? 'The vision model requested another edit, but no retries remain.'
            : 'The vision model could not confirm the result. Review it yourself.'
      return source?.error || decision !== 'accept'
    })
    if (!cancelled && failures.length)
      error.value = `Some views need attention: ${failures.join(', ')}. Check the selected photos for details.`
  } else if (!cancelled && previewed.value.length) {
    error.value = 'The vision model could not identify a view. Choose a photo manually to continue.'
    stage.value = 'select'
  }
  currentRun = undefined
  if (cancelled) error.value = 'Run cancelled.'
  running.value = false
  if (cancelled && previewed.value.length && !approved.value) stage.value = 'select'
}
async function extract() {
  const view = cutoutView.value
  const source = ready.value.find((c) => c.path === selection.value[view])
  if (
    !approved.value ||
    !source?.preview ||
    !visionReady.value ||
    !editReady.value ||
    (attempts.value[view] ?? 0) >= 2 ||
    running.value
  )
    return
  cancelled = false
  running.value = true
  started.value = Date.now()
  progress.value = `Extracting ${view}`
  error.value = ''
  const retry = (attempts.value[view] ?? 0) > 0
  try {
    currentRun = await invoke<number>('playground_start_run')
    if (cancelled) {
      await invoke('playground_cancel_run', { runId: currentRun })
      return
    }
    attempts.value[view] = (attempts.value[view] ?? 0) + 1
    const result = await invoke<Omit<Cutout, 'approved' | 'attempt'>>(
      'playground_extract_subject',
      {
        runId: currentRun,
        input: { image: source.preview, model: matteModel.value, retry },
      },
    )
    if (!cancelled) {
      if (cutouts.value[view]) previousCutouts.value[view] = cutouts.value[view]
      cutouts.value[view] = { ...result, approved: false, attempt: attempts.value[view]! }
      const candidate = cutouts.value[view]!
      progress.value = `Reviewing ${view} preview`
      const rendered = await invoke<{ original: string; composite: string }>(
        'portfolio_finish_preview',
        {
          input: {
            original: candidate.original,
            cutout: candidate.cutout,
            settings: { ...finishSettings.value },
          },
        },
      )
      if (cancelled) return
      currentRun = await invoke<number>('playground_start_run')
      if (cancelled) {
        await invoke('playground_cancel_run', { runId: currentRun })
        return
      }
      const review = await invoke<string>('portfolio_review', {
        runId: currentRun,
        input: { original: rendered.original, candidate: rendered.composite, model: model.value },
      })
      if (!cancelled) {
        candidate.review = review
        candidate.reviewModel = model.value
        if (reviewDecision(review) !== 'accept')
          error.value = 'The vision model could not confirm this preview. Review it yourself.'
      }
    }
  } catch (e) {
    if (!cancelled) error.value = String(e)
  } finally {
    currentRun = undefined
    if (cancelled) error.value = 'Run cancelled.'
    running.value = false
  }
}
function switchCandidate() {
  const view = cutoutView.value
  const previous = previousCutouts.value[view]
  const current = cutouts.value[view]
  if (running.value || !previous || !current) return
  cutouts.value[view] = previous
  previousCutouts.value[view] = current
}
async function reviewCutout() {
  const candidate = currentCutout.value
  if (!candidate || !finish.value || finishPending.value || !model.value || running.value) return
  cancelled = false
  running.value = true
  started.value = Date.now()
  progress.value = 'Reviewing candidate'
  error.value = ''
  try {
    currentRun = await invoke<number>('playground_start_run')
    if (cancelled) {
      await invoke('playground_cancel_run', { runId: currentRun })
      return
    }
    const review = await invoke<string>('portfolio_review', {
      runId: currentRun,
      input: {
        original: finish.value.original,
        candidate: finish.value.composite,
        model: model.value,
      },
    })
    if (!cancelled) {
      candidate.review = review
      candidate.reviewModel = model.value
    }
  } catch (e) {
    if (!cancelled) error.value = String(e)
  } finally {
    currentRun = undefined
    if (cancelled) error.value = 'Run cancelled.'
    running.value = false
  }
}
async function developRaw() {
  const view = cutoutView.value
  const source = ready.value.find((candidate) => candidate.path === selection.value[view])
  if (!approved.value || !source || running.value) return
  delete developed.value[view]
  running.value = true
  developing.value = true
  started.value = Date.now()
  progress.value = `Developing ${view}; choose a new PNG filename`
  error.value = ''
  try {
    const result = await invoke<DevelopedRaw | null>('portfolio_develop_raw', {
      raw: await readRawFile(source.file),
    })
    if (result) developed.value[view] = result
  } catch (e) {
    error.value = String(e)
  } finally {
    developing.value = false
    running.value = false
  }
}
async function exportComposite() {
  const view = cutoutView.value
  const source = ready.value.find((candidate) => candidate.path === selection.value[view])
  if (
    !source ||
    currentCutout.value?.model !== matteModel.value ||
    running.value ||
    !props.models.some((item) => item.id === matteModel.value && item.portraitMattingVerified)
  )
    return
  cancelled = false
  running.value = true
  started.value = Date.now()
  progress.value = `Exporting ${view}; choose a PNG filename`
  error.value = ''
  try {
    currentRun = await invoke<number>('playground_start_run')
    if (cancelled) {
      await invoke('playground_cancel_run', { runId: currentRun })
      return
    }
    const result = await invoke<DevelopedRaw | null>('portfolio_export_composite', {
      runId: currentRun,
      input: {
        raw: await readRawFile(source.file),
        model: matteModel.value,
        settings: { ...finishSettings.value },
      },
    })
    if (!cancelled && result) exported.value[view] = result
  } catch (e) {
    if (!cancelled) error.value = String(e)
  } finally {
    if (currentRun !== undefined)
      await invoke('playground_cancel_run', { runId: currentRun }).catch(() => {})
    currentRun = undefined
    if (cancelled) error.value = 'Export cancelled.'
    running.value = false
  }
}
</script>

<template>
  <PlaygroundFlowLayout
    v-model="mode"
    title="Portfolio Studio"
    settings-label="Portfolio Studio settings"
    result-label="Portfolio Studio result"
    graph-test-id="portfolio-graph"
    app-test-id="portfolio-app"
    :running="running"
    :started="started"
    :loading-label="progress"
    :loading-cancellable="!developing"
    :error="error"
    :steps="8"
    :graph-width="704"
    app-top
    @cancel="cancel"
  >
    <template #settings>
      <div>
        <h1 class="m-0 text-rune-title">Portfolio Studio</h1>
        <p class="text-rune-caption text-rune-text-2">
          Prepare and review portraits from your Sony ARW photos.
        </p>
      </div>
      <nav aria-label="Portfolio steps" class="flex flex-col gap-2">
        <RuneButton
          v-for="item in stages"
          :key="item.id"
          type="button"
          variant="ghost"
          :disabled="
            running ||
            (item.id === 'select' && !previewed.length) ||
            (item.id === 'create' && !approved)
          "
          class="justify-start! gap-3 rounded-rune-tile! px-3 py-3 text-left! text-rune-body normal-case! transition-colors disabled:opacity-40"
          :class="
            stage === item.id
              ? 'bg-rune-raised text-rune-ink'
              : 'text-rune-text-2 hover:bg-rune-raised'
          "
          :aria-current="stage === item.id ? 'step' : undefined"
          @click="stage = item.id"
        >
          <span class="text-rune-caption text-rune-text-3">{{ item.number }}</span
          >{{ item.label }}
        </RuneButton>
      </nav>
      <div class="border-t border-rune-text-3/20 pt-5">
        <p class="m-0 text-rune-heading text-rune-text-3 uppercase">Shoot folder</p>
        <label
          for="portfolio-folder"
          class="mt-3 inline-flex cursor-pointer rounded-full bg-rune-raised px-4 py-2 text-rune-heading"
          :class="{ 'pointer-events-none opacity-40': running }"
          >{{ candidates.length ? 'Change folder' : 'Choose folder' }}</label
        >
        <input
          id="portfolio-folder"
          type="file"
          webkitdirectory
          multiple
          class="sr-only"
          :disabled="running"
          @change="chooseFolder"
        />
        <p class="m-0 text-rune-caption text-rune-text-3">
          {{
            candidates.length
              ? `${ready.length} available · ${blocked.length} blocked`
              : 'Sony ARW files and their sidecars'
          }}
        </p>
      </div>
      <div class="flex flex-col gap-2 border-t border-rune-text-3/20 pt-5">
        <label for="portfolio-vision-model" class="text-rune-heading text-rune-text-3 uppercase"
          >Vision model</label
        >
        <div class="flex min-w-0 items-center gap-2">
          <ModelIcon v-if="model" :model="model" />
          <select
            id="portfolio-vision-model"
            v-model="model"
            :disabled="running"
            class="w-full rounded-rune-tile bg-rune-sunken p-3 text-rune-body"
          >
            <option v-for="option in options" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
        <p class="m-0 text-rune-caption text-rune-text-3">
          Classifies views and reviews previews. Choose a local vision model before running.
        </p>
      </div>
      <div class="flex flex-col gap-2 border-t border-rune-text-3/20 pt-5">
        <label for="portfolio-edit-model" class="text-rune-heading text-rune-text-3 uppercase"
          >Cutout model</label
        >
        <div class="flex min-w-0 items-center gap-2">
          <ModelIcon v-if="matteModel" :model="matteModel" />
          <select
            id="portfolio-edit-model"
            v-model="matteModel"
            aria-label="Cutout model"
            :disabled="running"
            class="w-full rounded-rune-tile bg-rune-sunken p-3 text-rune-body"
          >
            <option
              v-for="option in matteOptions"
              :key="option.value"
              :value="option.value"
              :disabled="option.disabled"
            >
              {{ option.label }}
            </option>
          </select>
        </div>
        <p class="m-0 text-rune-caption text-rune-text-3">
          Choose a model before running. Cutouts are previews; matte detail depends on the model's
          inference resolution.
        </p>
      </div>
      <div class="flex flex-col gap-3 border-t border-rune-text-3/20 pt-5">
        <p class="m-0 text-rune-heading text-rune-text-3 uppercase">Shoot look</p>
        <p class="m-0 text-rune-caption text-rune-text-3">
          These settings apply to each selected view in this session. Adjusting them clears previous
          approvals and reviews. Exposure and warmth are approximate adjustments; the RAW remains
          unchanged.
        </p>
        <label for="portfolio-crop" class="text-rune-caption">Crop</label>
        <select
          id="portfolio-crop"
          v-model="finishSettings.crop"
          :disabled="running"
          class="w-full rounded-rune-tile bg-rune-sunken p-3 text-rune-body"
        >
          <option value="original">Original</option>
          <option value="portrait">Portrait 2:3</option>
          <option value="square">Square 1:1</option>
        </select>
        <label class="flex items-center gap-2 text-rune-caption">
          <input v-model="finishSettings.autoCenter" type="checkbox" :disabled="running" />
          Center subject horizontally
        </label>
        <label class="flex items-center gap-2 text-rune-caption">
          <input v-model="automaticLook" type="checkbox" :disabled="running" />
          Estimate exposure and warmth from selected RAW photos
        </label>
        <p class="m-0 text-rune-caption text-rune-text-3">
          Selected RAW files are developed for accurate previews. Automatic adjustments are
          approximate and can be edited.
        </p>
        <label
          v-for="slider in finishSliders"
          :key="slider.key"
          class="flex flex-col gap-1 text-rune-caption"
        >
          <span class="flex justify-between gap-2"
            ><span>{{ slider.label }}</span
            ><output>{{
              slider.key === 'exposure'
                ? `${(finishSettings.exposure / 100).toFixed(2)} EV`
                : finishSettings[slider.key]
            }}</output></span
          >
          <input
            v-model.number="finishSettings[slider.key]"
            type="range"
            :min="slider.min"
            :max="slider.max"
            :step="slider.step"
            :disabled="running"
            :aria-label="slider.label"
            class="w-full"
          />
        </label>
      </div>
    </template>
    <template #graph>
      <PlaygroundFlowNode
        step="01"
        kind="Input"
        title="Shoot folder"
        :status="`${candidates.length} files`"
        icon="image"
        tone="sky"
        output
        class="max-w-[320px]"
      />
      <PlaygroundFlowConnection label="RAW files" tone="sky" />
      <PlaygroundFlowNode
        step="02"
        kind="Operation"
        title="Check sidecars and prepare previews"
        :status="running ? 'Running' : 'Ready'"
        icon="scan"
        tone="violet"
        input
        output
        class="max-w-[320px]"
      />
      <PlaygroundFlowConnection label="Eligible images" tone="violet" />
      <PlaygroundFlowNode
        step="03"
        kind="Operation"
        title="Suggest views"
        :status="model || 'Select a vision model'"
        icon="scan"
        tone="violet"
        input
        output
        class="max-w-[320px]"
      />
      <PlaygroundFlowConnection label="Candidates" tone="violet" />
      <PlaygroundFlowNode
        step="04"
        kind="Review"
        title="Select views"
        :status="approved ? 'Ready' : 'Waiting'"
        icon="image"
        tone="emerald"
        input
        output
        class="max-w-[320px]"
      />
      <PlaygroundFlowConnection label="Selected source" tone="violet" />
      <div class="relative flex w-full justify-center gap-8">
        <span
          class="absolute top-0 left-1/4 right-1/4 border-t border-dashed border-rune-text-3/50"
          aria-hidden="true"
        ></span>
        <div class="flex w-80 flex-col items-center">
          <span class="h-12 border-l border-dashed border-rune-text-3/50" aria-hidden="true"></span>
          <PlaygroundFlowNode
            step="05"
            kind="Operation"
            title="Develop 16-bit source PNG"
            :status="developing ? 'Running' : developed[cutoutView] ? 'Saved' : 'Optional'"
            icon="image"
            tone="violet"
            input
          />
        </div>
        <div class="flex w-80 flex-col items-center">
          <span class="h-12 border-l border-dashed border-rune-text-3/50" aria-hidden="true"></span>
          <PlaygroundFlowNode
            step="06"
            kind="Operation"
            title="Extract subject and composite"
            :status="
              running && !developing && progress.startsWith('Extracting') ? 'Running' : 'Ready'
            "
            icon="image"
            tone="violet"
            input
            output
          />
          <PlaygroundFlowConnection label="Cutout preview" tone="violet" />
          <PlaygroundFlowNode
            step="07"
            kind="Operation"
            title="Vision review"
            :status="
              currentCutout?.review
                ? reviewDecision(currentCutout.review) === 'accept'
                  ? 'Accepted'
                  : 'Needs review'
                : 'Waiting'
            "
            icon="scan"
            tone="violet"
            input
            output
          />
          <PlaygroundFlowConnection label="Review notes" tone="violet" />
          <PlaygroundFlowNode
            step="08"
            kind="Output"
            title="Results and optional review"
            :status="
              currentCutout?.approved
                ? 'Reviewed'
                : currentCutout?.review
                  ? reviewDecision(currentCutout.review) === 'accept'
                    ? 'AI reviewed'
                    : 'Needs review'
                  : 'Waiting'
            "
            icon="image"
            tone="emerald"
            input
          />
        </div>
      </div>
    </template>
    <template #app>
      <section v-if="stage === 'import'" class="w-full space-y-5" aria-label="Import photos">
        <div class="rounded-rune-tile border border-white/10 bg-rune-raised p-6 sm:p-8">
          <p class="m-0 text-rune-heading text-rune-text-3 uppercase">Step 01 · Import</p>
          <h2 class="mt-3 mb-2 text-rune-title">Start with your shoot</h2>
          <p class="m-0 max-w-2xl text-rune-body text-rune-text-2">
            Choose a folder of Sony ARW photos. Studio checks for XMP edits, then prepares previews
            so you can choose your views.
          </p>
          <div class="mt-6 rounded-rune-tile bg-rune-sunken p-4">
            <p class="m-0 text-rune-body">
              {{ candidates.length ? `${ready.length} photos available` : 'No folder selected' }}
            </p>
            <p class="mt-1 mb-0 text-rune-caption text-rune-text-3">
              {{
                candidates.length
                  ? `${blocked.length} blocked by XMP edits`
                  : 'Choose a folder to get started.'
              }}
            </p>
          </div>
          <label
            v-if="!candidates.length"
            for="portfolio-folder"
            class="mt-5 inline-flex cursor-pointer rounded-full bg-rune-ink px-5 py-3 text-rune-heading text-black"
            :class="{ 'pointer-events-none opacity-40': running }"
            >Choose shoot folder</label
          >
          <RuneButton
            v-if="previewed.length"
            variant="primary"
            class="mt-5"
            :disabled="running"
            @click="stage = 'select'"
            >Continue to choose views</RuneButton
          >
          <RuneButton
            v-else-if="candidates.length"
            variant="primary"
            class="mt-5"
            :disabled="running || !ready.length || !visionReady || !editReady"
            @click="run()"
            >Run automated previews</RuneButton
          >
          <p v-if="candidates.length && !ready.length" class="text-rune-caption text-rune-text-3">
            No eligible photos found. Choose a folder with unedited ARW files.
          </p>
          <p
            v-if="candidates.length && (!visionReady || !editReady)"
            class="text-rune-caption text-rune-text-3"
          >
            Select a local vision model and a cutout model in the left panel to run.
          </p>
          <p class="mt-4 mb-0 text-rune-caption text-rune-text-3">
            The folder picker may ask you to confirm access to its files. Your originals remain
            unchanged.
          </p>
        </div>
        <details
          v-if="blocked.length"
          class="rounded-rune-tile border border-amber-400/20 bg-rune-raised p-4 text-rune-caption"
          role="status"
        >
          <summary class="cursor-pointer">
            {{ blocked.length }} files blocked by XMP edits · View files
          </summary>
          <p class="text-rune-text-2">These edits cannot yet be faithfully rendered.</p>
          <p v-for="item in blocked" :key="item.path" class="mb-0 break-all">{{ item.path }}</p>
        </details>
        <details
          v-if="ready.some((item) => item.error)"
          class="rounded-rune-tile bg-rune-raised p-4 text-rune-caption text-error"
          open
        >
          <summary>Some photos could not be prepared</summary>
          <p v-for="item in ready.filter((item) => item.error)" :key="item.path">
            {{ item.file.name }}: {{ item.error }}
          </p>
        </details>
      </section>

      <section v-if="stage === 'select'" class="w-full space-y-5" aria-label="Choose views">
        <div class="rounded-rune-tile border border-white/10 bg-rune-raised p-6">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div>
              <p class="m-0 text-rune-heading text-rune-text-3 uppercase">Step 02 · Choose views</p>
              <h2 class="mt-2 mb-1 text-rune-title">Build your selection</h2>
              <p class="m-0 text-rune-caption text-rune-text-2">
                Choose a view, then click a photo below. You can leave views empty.
              </p>
            </div>
            <span class="rounded-full bg-rune-sunken px-3 py-2 text-rune-caption"
              >{{ selectedCount }} of {{ views.length }} selected</span
            >
          </div>
          <div class="mt-5 grid gap-2 sm:grid-cols-2">
            <div
              v-for="view in views"
              :key="view"
              class="flex min-w-0 flex-col gap-2 rounded-rune-tile bg-rune-sunken p-3 text-rune-caption"
              :class="activeView === view ? 'ring-1 ring-white/30' : ''"
            >
              <RuneButton
                type="button"
                variant="link"
                class="justify-start! text-left! capitalize normal-case!"
                :aria-pressed="activeView === view"
                @click="activeView = view"
              >
                {{ view
                }}<span v-if="activeView === view" class="ml-2 text-rune-text-3">· active</span>
              </RuneButton>
              <select
                v-model="selection[view]"
                :aria-label="`${view} photo`"
                :disabled="running"
                class="min-w-0 rounded-rune-tile bg-rune-raised p-2"
                @focus="activeView = view"
                @change="selectionChanged(view)"
              >
                <option value="">Not selected</option>
                <option v-for="item in previewed" :key="item.path" :value="item.path">
                  {{ item.file.name }}
                </option>
              </select>
            </div>
          </div>
          <div class="mt-5 flex flex-wrap items-center gap-3">
            <RuneButton
              variant="primary"
              :disabled="running || !selectedCount || !visionReady || !editReady"
              @click="run(true)"
              >Continue automatically</RuneButton
            >
            <span class="text-rune-caption text-rune-text-3">{{
              selectedCount
                ? 'You can change these later.'
                : 'Select at least one photo to continue.'
            }}</span>
          </div>
        </div>
        <div>
          <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
            <h3 class="m-0 text-rune-body">
              Photos <span class="text-rune-text-3">· {{ previewed.length }}</span>
            </h3>
            <span class="text-rune-caption text-rune-text-3"
              >Click a photo to assign it to {{ activeView }}.</span
            >
          </div>
          <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            <RuneButton
              v-for="item in previewed"
              :key="item.path"
              type="button"
              variant="ghost"
              :disabled="running"
              :aria-label="`Assign ${item.file.name} to ${activeView}`"
              :aria-pressed="selection[activeView] === item.path"
              class="min-w-0 flex-col! items-stretch! gap-0! overflow-hidden rounded-rune-tile! border bg-rune-raised p-0! text-left! normal-case!"
              :class="
                selection[activeView] === item.path
                  ? 'border-white/70'
                  : 'border-white/10 hover:border-white/40'
              "
              @click="assignPhoto(item.path)"
            >
              <img
                :src="item.preview"
                :alt="item.file.name"
                class="aspect-[3/4] w-full object-contain bg-rune-sunken"
              />
              <span class="block truncate px-3 pt-2 text-rune-caption">{{ item.file.name }}</span>
              <span class="block truncate px-3 pb-2 text-rune-caption text-rune-text-3">{{
                item.suggestion ? `Suggested: ${item.suggestion}` : 'Choose for active view'
              }}</span>
            </RuneButton>
          </div>
          <details
            v-if="ready.some((item) => item.error)"
            class="mt-4 text-rune-caption text-error"
          >
            <summary>Some photos could not be prepared</summary>
            <p v-for="item in ready.filter((item) => item.error)" :key="item.path">
              {{ item.file.name }}: {{ item.error }}
            </p>
          </details>
          <RuneButton
            v-if="ready.some((item) => !item.preview || item.error)"
            class="mt-3"
            :disabled="running"
            @click="run()"
            >Prepare remaining photos</RuneButton
          >
        </div>
      </section>

      <section
        v-if="stage === 'create' && approved"
        aria-label="Subject cutout"
        class="w-full space-y-5"
      >
        <div
          v-if="ready.some((item) => item.error)"
          class="rounded-rune-tile border border-amber-400/20 bg-rune-raised p-4 text-rune-caption"
          role="status"
        >
          <p class="m-0">Some photos need attention:</p>
          <p
            v-for="item in ready.filter((item) => item.error)"
            :key="item.path"
            class="mb-0 break-words"
          >
            {{ item.file.name }}: {{ item.error }}
          </p>
        </div>
        <div class="rounded-rune-tile border border-white/10 bg-rune-raised p-6">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p class="m-0 text-rune-heading text-rune-text-3 uppercase">
                Step 03 · Create & review
              </p>
              <h2 class="mt-2 mb-1 text-rune-title">Create a cutout preview</h2>
              <p class="m-0 text-rune-caption text-rune-text-2">
                The vision model reviews each preview automatically. Choose a view to see its
                result.
              </p>
            </div>
            <RuneButton variant="ghost" :disabled="running" @click="stage = 'select'"
              >Edit selections</RuneButton
            >
          </div>
          <div class="mt-5 grid gap-3">
            <label class="flex flex-col gap-2 text-rune-caption"
              >Selected view
              <select
                v-model="cutoutView"
                aria-label="Cutout view"
                :disabled="running"
                class="rounded-rune-tile bg-rune-sunken p-3"
              >
                <option v-for="view in views.filter((v) => selection[v])" :key="view" :value="view">
                  {{ view }} · {{ ready.find((item) => item.path === selection[view])?.file.name }}
                </option>
              </select>
            </label>
          </div>
          <div class="mt-5 flex flex-wrap items-center gap-3">
            <RuneButton
              variant="primary"
              :disabled="
                running ||
                !selection[cutoutView] ||
                !visionReady ||
                !editReady ||
                (attempts[cutoutView] ?? 0) >= 2
              "
              @click="extract"
              >{{
                (attempts[cutoutView] ?? 0) ? 'Retry from original' : 'Create cutout preview'
              }}</RuneButton
            >
            <span class="text-rune-caption text-rune-text-3">{{
              !visionReady
                ? 'Choose a local vision model in the left panel to continue.'
                : !editReady
                  ? 'Choose a cutout model in the left panel to continue.'
                  : `Attempt ${attempts[cutoutView] ?? 0} of 2 · preview only`
            }}</span>
          </div>
          <details class="mt-5 rounded-rune-tile bg-rune-sunken p-4 text-rune-caption">
            <summary class="cursor-pointer">Develop original as 16-bit PNG</summary>
            <p class="text-rune-text-2">
              Save the full-resolution source without the cutout or XMP edits. Color and crop may
              differ from the preview; camera metadata is not copied. Choose a new filename.
            </p>
            <RuneButton :disabled="running || !selection[cutoutView]" @click="developRaw"
              >Develop 16-bit PNG</RuneButton
            >
            <p v-if="developed[cutoutView]" role="status" class="mb-0 break-all">
              Saved {{ developed[cutoutView]?.width }} × {{ developed[cutoutView]?.height }} PNG to
              {{ developed[cutoutView]?.path }}
            </p>
          </details>
          <div
            v-if="
              props.models.some((item) => item.id === matteModel && item.portraitMattingVerified)
            "
            class="mt-5 rounded-rune-tile bg-rune-sunken p-4"
          >
            <p class="m-0 text-rune-caption text-rune-text-2">
              Export the full-resolution developed RAW with a new matte and the selected background
              gradient and the selected crop and adjustments. Edge refinement may use multiple model
              passes and take longer. Vision feedback applies to the preview; inspect the exported
              image.
            </p>
            <RuneButton
              class="mt-3"
              :disabled="
                running ||
                !selection[cutoutView] ||
                !currentCutout ||
                currentCutout.model !== matteModel ||
                (!currentCutout.approved && reviewDecision(currentCutout.review) !== 'accept')
              "
              @click="exportComposite"
              >Export full-resolution composite</RuneButton
            >
            <p v-if="exported[cutoutView]" role="status" class="mb-0 break-all text-rune-caption">
              Saved {{ exported[cutoutView]?.width }} × {{ exported[cutoutView]?.height }} PNG to
              {{ exported[cutoutView]?.path }}
            </p>
          </div>
        </div>

        <div
          v-if="currentCutout"
          class="rounded-rune-tile border border-white/10 bg-rune-raised p-6"
        >
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="m-0 text-rune-heading text-rune-text-3 uppercase">
                Review result · Candidate {{ currentCutout.attempt }}
              </p>
              <p class="mt-1 mb-0 text-rune-caption text-rune-text-2">
                Compare the edges and details. This is a preview, not a full-resolution export.
              </p>
            </div>
            <RuneButton
              v-if="previousCutouts[cutoutView]"
              :disabled="running"
              @click="switchCandidate"
              >Show other candidate</RuneButton
            >
          </div>
          <div class="mt-4 flex flex-wrap items-center gap-3">
            <RuneButton
              v-if="!currentCutout.approved"
              variant="secondary"
              :disabled="running || finishPending || !finish"
              @click="currentCutout.approved = true"
              >Mark reviewed (optional)</RuneButton
            >
            <a
              v-if="
                (currentCutout.approved || reviewDecision(currentCutout.review) === 'accept') &&
                !running
              "
              :href="currentCutout.cutout"
              download="portfolio-cutout-preview.png"
              class="rounded-full bg-rune-sunken px-4 py-2 text-rune-heading"
              >Download unadjusted transparent preview</a
            >
            <a
              v-if="
                (currentCutout.approved || reviewDecision(currentCutout.review) === 'accept') &&
                finish &&
                !running
              "
              :href="finish.composite"
              download="portfolio-look-preview.png"
              class="rounded-full bg-rune-sunken px-4 py-2 text-rune-heading"
              >Download look preview</a
            >
            <RuneButton
              :disabled="running || finishPending || !finish || !model"
              @click="reviewCutout"
              >Review with vision model</RuneButton
            >
            <span v-if="!model" class="text-rune-caption text-rune-text-3"
              >Choose a vision model on the left for feedback.</span
            >
            <span
              v-if="currentCutout.approved"
              role="status"
              class="text-rune-caption text-rune-text-2"
              >Marked reviewed for this session</span
            >
          </div>
          <div
            v-if="currentCutout.review"
            class="mt-4 rounded-rune-tile bg-rune-sunken p-4 text-rune-caption"
          >
            <p class="m-0">
              Review by {{ currentCutout.reviewModel }} · AI feedback can be inaccurate
            </p>
            <p class="mb-0 whitespace-pre-wrap text-rune-text-2">{{ currentCutout.review }}</p>
          </div>
          <p v-if="finishPending" role="status" class="text-rune-caption text-rune-text-3">
            Updating look preview…
          </p>
          <p v-if="finishError" role="alert" class="text-rune-caption text-error">
            {{ finishError }}
          </p>
          <div v-if="finish" class="mt-5 grid gap-4 sm:grid-cols-2">
            <figure class="m-0">
              <figcaption class="mb-2 text-rune-caption text-rune-text-2">
                Original preview · same framing
              </figcaption>
              <img
                :src="finish.original"
                alt="Original preview"
                class="max-h-[520px] w-full rounded-rune-tile bg-rune-sunken object-contain"
              />
            </figure>
            <figure class="m-0">
              <figcaption class="mb-2 text-rune-caption text-rune-text-2">
                Studio background preview
              </figcaption>
              <img
                :src="finish.composite"
                alt="Studio background preview"
                class="max-h-[520px] w-full rounded-rune-tile bg-rune-sunken object-contain"
              />
            </figure>
          </div>
          <details class="mt-4 text-rune-caption">
            <summary class="cursor-pointer">Inspect transparent cutout</summary>
            <img
              :src="currentCutout.cutout"
              alt="Transparent subject cutout"
              class="mt-2 w-full max-w-sm rounded-rune-tile"
              style="background: repeating-conic-gradient(#777 0% 25%, #bbb 0% 50%) 0 0 / 20px 20px"
            />
          </details>
          <p class="mb-0 text-rune-caption text-rune-text-3">
            {{ currentCutout.model }} · {{ (currentCutout.elapsedMs / 1000).toFixed(1) }} s
          </p>
        </div>
      </section>
    </template>
  </PlaygroundFlowLayout>
</template>