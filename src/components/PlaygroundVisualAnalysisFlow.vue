<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useAnalysis } from '@/composables/useAnalysis'
import { readSnapshot } from '@/config/analysis'
import { friendly } from '@/config/errors'
import { taskModelIds, type DepthResult, type DepthState } from '@/config/playground'
import type { AnalysisSnapshot, ModelCapabilities } from '@/types/protocol'
import CaptureTargetPicker from './CaptureTargetPicker.vue'
import PlaygroundDepth from './PlaygroundDepth.vue'
import PlaygroundFlowLayout from './PlaygroundFlowLayout.vue'
import PlaygroundFlowNode from './PlaygroundFlowNode.vue'
import PlaygroundFlowConnection from './PlaygroundFlowConnection.vue'
import PlaygroundImagePicker from './PlaygroundImagePicker.vue'
import PlaygroundResult from './PlaygroundResult.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
import RuneSwitch from './RuneSwitch.vue'

const props = defineProps<{ models: ModelCapabilities[] }>()
type RoleId = 'entities' | 'regions' | 'structure' | 'depth'
type QuestionRoleId = Exclude<RoleId, 'depth'>
type RoleState = {
  status: 'waiting' | 'running' | 'done' | 'error' | 'cancelled'
  error: string
  snapshot: AnalysisSnapshot | null
  depth: DepthResult | null
}
interface TargetOption {
  target: { kind: string; id: number }
  label: string
  primary?: boolean
}
const definitions: { id: RoleId; label: string }[] = [
  { id: 'entities', label: 'Entities' },
  { id: 'regions', label: 'Regions & Context' },
  { id: 'structure', label: 'Structure & Landmarks' },
  { id: 'depth', label: 'Depth' },
]
const blankRole = (): RoleState => ({ status: 'waiting', error: '', snapshot: null, depth: null })
const roles = ref<Record<RoleId, { enabled: boolean; model: string }>>({
  entities: { enabled: true, model: '' },
  regions: { enabled: true, model: '' },
  structure: { enabled: true, model: '' },
  depth: { enabled: true, model: '' },
})
const results = ref<Record<RoleId, RoleState>>({
  entities: blankRole(),
  regions: blankRole(),
  structure: blankRole(),
  depth: blankRole(),
})
const visionModels = computed(() =>
  props.models
    .filter((model) => model.imageIn === 'yes' && model.toolCalls === 'yes')
    .map((model) => model.id),
)
const depthModels = computed(() => taskModelIds(props.models, 'image-to-depth'))
const options = (role: RoleId) => [
  { value: '', label: 'Select a model', disabled: true },
  ...(role === 'depth' ? depthModels.value : visionModels.value).map((id) => ({
    value: id,
    label: id,
  })),
]
const enabledRoles = computed(() => definitions.filter(({ id }) => roles.value[id].enabled))
const analysis = useAnalysis()
const question = ref('')
const source = ref<'screen' | 'image'>('screen')
const image = ref<{ name: string; data: string } | null>(null)
const sources = [
  { value: 'screen' as const, label: 'Screen or window' },
  { value: 'image' as const, label: 'Image' },
]
const mode = ref<'app' | 'graph'>('app')
const plannerModel = ref('')
const plannerStatus = ref<'waiting' | 'running' | 'done' | 'error' | 'cancelled'>('waiting')
const plannerQuestions = ref<Record<QuestionRoleId, string> | null>(null)
const runRoles = ref<RoleId[]>([])
const selectedResult = ref<RoleId>('entities')
function navigateResults(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key) || !runRoles.value.length)
    return
  event.preventDefault()
  const index = runRoles.value.indexOf(selectedResult.value)
  const next =
    event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? runRoles.value.length - 1
        : (index + (event.key === 'ArrowRight' ? 1 : -1) + runRoles.value.length) %
          runRoles.value.length
  selectedResult.value = runRoles.value[next]!
  const list = (event.currentTarget as HTMLElement).parentElement
  void nextTick(() =>
    (list?.querySelector('[role="tab"][aria-selected="true"]') as HTMLElement | null)?.focus(),
  )
}
const targets = ref<TargetOption[]>([])
const target = ref('')
const refreshing = ref(false)
const targetError = ref('')
const runError = ref('')
const running = ref(false)
const started = ref(0)
const capturedImage = ref('')
const lastConfiguration = ref('')
const selectedTarget = computed(() =>
  targets.value.find((item) => JSON.stringify(item.target) === target.value),
)
const configuration = computed(() =>
  JSON.stringify({
    source: source.value,
    target: source.value === 'screen' ? target.value : '',
    image: source.value === 'image' ? image.value?.data : '',
    question: question.value.trim(),
    roles: roles.value,
    planner: plannerModel.value,
  }),
)
const previousResult = computed(
  () => !!lastConfiguration.value && lastConfiguration.value !== configuration.value,
)
const canRun = computed(
  () =>
    !running.value &&
    !analysis.busy.value &&
    enabledRoles.value.length > 0 &&
    visionModels.value.includes(plannerModel.value) &&
    (source.value === 'image' ? !!image.value : !!selectedTarget.value) &&
    enabledRoles.value.every(({ id }) =>
      (id === 'depth' ? depthModels.value : visionModels.value).includes(roles.value[id].model),
    ),
)
const loadingLabel = computed(() =>
  plannerStatus.value === 'running' ? 'Planning questions' : 'Analyzing branches',
)
let active: { cancelled: boolean; ids: Set<number> } | null = null

async function loadTargets() {
  if (refreshing.value) return
  refreshing.value = true
  targetError.value = ''
  try {
    const result = await invoke<unknown>('screen_insight_targets')
    targets.value = Array.isArray(result) ? result : []
    if (!targets.value.some((item) => JSON.stringify(item.target) === target.value))
      target.value = ''
    if (!target.value) {
      const primary = targets.value.find((item) => item.primary && item.target.kind === 'screen')
      if (primary) target.value = JSON.stringify(primary.target)
    }
    if (!targets.value.length)
      targetError.value = 'No capture targets available. Check screen permissions and refresh.'
  } catch (error) {
    targets.value = []
    target.value = ''
    targetError.value = friendly(String(error))
  } finally {
    refreshing.value = false
  }
}
onMounted(loadTargets)
function cancel() {
  if (!active) return
  active.cancelled = true
  for (const id of active.ids) void invoke('playground_cancel_run', { runId: id }).catch(() => {})
}
onUnmounted(cancel)
async function run() {
  if (!canRun.value) return
  const selected = selectedTarget.value
  const selectedRoles = enabledRoles.value.map((role) => ({
    ...role,
    model: roles.value[role.id].model,
  }))
  runRoles.value = selectedRoles.map((role) => role.id)
  selectedResult.value = selectedRoles[0]!.id
  const request = { cancelled: false, ids: new Set<number>() }
  active = request
  running.value = true
  started.value = Date.now()
  runError.value = ''
  lastConfiguration.value = configuration.value
  capturedImage.value = ''
  plannerQuestions.value = null
  plannerStatus.value = 'running'
  results.value = {
    entities: blankRole(),
    regions: blankRole(),
    structure: blankRole(),
    depth: blankRole(),
  }
  async function call<T>(command: string, input: unknown): Promise<T> {
    const runId = await invoke<number>('playground_start_run')
    if (request.cancelled) {
      await invoke('playground_cancel_run', { runId })
      throw new Error('Flow cancelled.')
    }
    request.ids.add(runId)
    try {
      return await invoke<T>(command, { runId, input })
    } finally {
      request.ids.delete(runId)
    }
  }
  try {
    const input =
      source.value === 'image' ? { image: image.value?.data } : { target: selected?.target }
    const plan = await call<{
      image: string
      model: string
      questions: Record<QuestionRoleId, string>
    }>('playground_visual_plan', {
      ...input,
      model: plannerModel.value,
      question: question.value.trim(),
    })
    if (request.cancelled) {
      plannerStatus.value = 'cancelled'
      return
    }
    capturedImage.value = plan.image
    plannerQuestions.value = plan.questions
    plannerStatus.value = 'done'
    await Promise.all(
      selectedRoles.map(async (role) => {
        const state = results.value[role.id]
        state.status = 'running'
        try {
          if (role.id === 'depth') {
            const depth = await call<DepthResult>('playground_depth', {
              image: plan.image,
              model: role.model,
            })
            if (request.cancelled) {
              state.status = 'cancelled'
              return
            }
            state.depth = depth
          } else {
            const result = await call<{
              answer: string
              findings: unknown[]
              model: string
              boxesRejected: number
            }>('playground_visual_branch', {
              image: plan.image,
              model: role.model,
              question: plan.questions[role.id],
            })
            if (request.cancelled) {
              state.status = 'cancelled'
              return
            }
            state.snapshot = readSnapshot({
              id: 'visual-' + role.id,
              status: 'done',
              screenshot: plan.image,
              answer: result.answer,
              findings: result.findings,
              model: result.model,
              question: plan.questions[role.id],
              boxesRejected: result.boxesRejected,
            })
            state.snapshot.findings = state.snapshot.findings.filter(
              (finding) => finding.group === role.id,
            )
          }
          state.status = request.cancelled ? 'cancelled' : 'done'
        } catch (error) {
          state.status = request.cancelled ? 'cancelled' : 'error'
          if (!request.cancelled)
            state.error = friendly(error instanceof Error ? error.message : String(error))
        }
      }),
    )
  } catch (error) {
    plannerStatus.value = request.cancelled ? 'cancelled' : 'error'
    if (!request.cancelled)
      runError.value = friendly(error instanceof Error ? error.message : String(error))
  } finally {
    if (request.cancelled) runError.value = 'Flow cancelled.'
    if (active === request) active = null
    running.value = false
  }
}

const roleStatus = (id: RoleId) =>
  ({
    waiting: 'Waiting',
    running: 'Running',
    done: 'Complete',
    error: 'Failed',
    cancelled: 'Cancelled',
  })[results.value[id].status]
const outputStatus = computed(() => {
  if (running.value) return 'Running'
  if (plannerStatus.value === 'error') return 'Failed'
  if (plannerStatus.value === 'cancelled') return 'Cancelled'
  const values = Object.values(results.value).map((result) => result.status)
  if (values.includes('done'))
    return values.includes('error') || runError.value ? 'Partial' : 'Complete'
  if (values.includes('error')) return 'Failed'
  if (values.includes('cancelled')) return 'Cancelled'
  return 'Waiting'
})
const depthState = (state: RoleState): DepthState => ({
  status: state.status === 'done' ? 'done' : 'error',
  result: state.depth,
  error: state.error,
})
</script>

<template>
  <PlaygroundFlowLayout
    v-model="mode"
    data-testid="visual-analysis-flow"
    title="Visual Analysis"
    settings-label="Visual Analysis settings"
    result-label="Visual Analysis result"
    graph-test-id="visual-flow-graph"
    app-test-id="visual-flow-app"
    :running="running"
    :started="started"
    :loading-label="loadingLabel"
    :error="runError"
    :stale="previousResult"
    :steps="enabledRoles.length + 3"
    graph-wide
    app-top
    @cancel="cancel"
  >
    <template #settings>
      <div>
        <h1 class="m-0 text-rune-title">Visual Analysis</h1>
        <p class="text-rune-body text-rune-text-2">Analyze one image using selected models.</p>
      </div>
      <div class="flex flex-col gap-2">
        <span class="text-rune-heading text-rune-text-3 uppercase">Source</span
        ><RuneChoice
          v-model="source"
          label="Visual Analysis source"
          :options="sources"
          :disabled="running"
        />
      </div>
      <div v-if="source === 'screen'" class="flex flex-col gap-2">
        <span class="text-rune-heading text-rune-text-3 uppercase">Screen or window</span
        ><CaptureTargetPicker
          v-model="target"
          :options="targets"
          :refreshing="refreshing"
          :disabled="running"
          contained
          @refresh="loadTargets"
        />
        <p v-if="targetError" role="alert" class="m-0 text-rune-caption text-error">
          {{ targetError }}
        </p>
      </div>
      <PlaygroundImagePicker
        v-else
        id="visual-flow-image"
        v-model="image"
        label="Visual Analysis image"
        :disabled="running"
      />
      <div class="flex flex-col gap-2">
        <label for="visual-flow-question" class="text-rune-heading text-rune-text-3 uppercase"
          >Question (optional)</label
        ><textarea
          id="visual-flow-question"
          v-model="question"
          maxlength="400"
          rows="4"
          :disabled="running"
          class="w-full resize-none rounded-rune-tile bg-rune-surface p-4 text-rune-body text-rune-ink"
          placeholder="What would you like to know?"
        />
        <p class="m-0 text-rune-caption text-rune-text-3">Up to 400 characters.</p>
      </div>
      <div class="flex flex-col gap-2">
        <span class="text-rune-heading text-rune-text-3 uppercase">Planning model</span>
        <RuneChoice
          v-model="plannerModel"
          label="Planning model"
          :options="options('entities')"
          :disabled="running"
          menu
          floating
        />
        <p class="m-0 text-rune-caption text-rune-text-3">
          The planning model creates questions for the text-based analysis branches.
        </p>
      </div>
      <div v-for="role in definitions" :key="role.id" class="flex flex-col gap-2">
        <div class="flex items-center justify-between gap-2">
          <span class="text-rune-heading text-rune-text-3 uppercase">{{ role.label }}</span
          ><RuneSwitch
            v-model="roles[role.id].enabled"
            :label="`Enable ${role.label}`"
            :disabled="running"
          />
        </div>
        <RuneChoice
          v-if="roles[role.id].enabled"
          v-model="roles[role.id].model"
          :label="`${role.label} model`"
          :options="options(role.id)"
          :disabled="running"
          menu
          floating
        />
        <p
          v-if="roles[role.id].enabled && options(role.id).length === 1"
          class="m-0 text-rune-caption text-rune-text-3"
        >
          {{
            role.id === 'depth'
              ? 'No depth model available.'
              : 'No model advertises image input and tool calling.'
          }}
        </p>
      </div>
      <p class="m-0 text-rune-caption text-rune-text-3">
        Depth produces a relative depth map without a question.
      </p>
      <p v-if="!enabledRoles.length" class="m-0 text-rune-caption text-rune-text-3">
        Enable at least one role.
      </p>
      <RuneButton variant="primary" :disabled="!canRun" @click="run">Run Flow</RuneButton>
    </template>
    <template #graph>
      <PlaygroundFlowNode
        step="01"
        kind="Input"
        :title="source === 'image' ? 'Image' : 'Screen or window'"
        :status="(source === 'image' ? image : selectedTarget) ? 'Ready' : 'Not set'"
        icon="image"
        tone="sky"
        output
        class="max-w-[320px]"
        data-testid="visual-node-input"
        ><p
          class="mt-4 mb-0 break-all rounded-rune-tile border border-white/5 bg-black/20 p-3 text-rune-caption text-rune-text-2"
        >
          {{
            source === 'image'
              ? (image?.name ?? 'Choose an image')
              : (selectedTarget?.label ?? 'Choose a screen or window')
          }}
        </p></PlaygroundFlowNode
      >
      <PlaygroundFlowConnection label="Image" tone="sky" />
      <PlaygroundFlowNode
        step="02"
        kind="Operation"
        title="Plan questions"
        :status="
          {
            waiting: 'Waiting',
            running: 'Running',
            done: 'Complete',
            error: 'Failed',
            cancelled: 'Cancelled',
          }[plannerStatus]
        "
        icon="scan"
        tone="violet"
        :active="plannerStatus === 'running'"
        input
        output
        class="max-w-[320px]"
        data-testid="visual-node-planner"
      >
        <div class="mt-4 rounded-rune-tile border border-white/5 bg-black/20 p-3">
          <p class="m-0 text-rune-caption text-rune-text-3">Planning model</p>
          <p class="mt-1 mb-0 break-words text-rune-body">
            {{ plannerModel || 'Select a planning model' }}
          </p>
        </div>
      </PlaygroundFlowNode>
      <PlaygroundFlowConnection label="Questions + image" tone="violet" />
      <div
        class="w-full overflow-x-auto pb-2"
        aria-label="Parallel analysis branches"
        data-testid="visual-parallel-branches"
      >
        <div class="mx-auto flex w-max min-w-full items-stretch justify-center">
          <div
            v-for="(role, index) in enabledRoles"
            :key="role.id"
            class="relative flex w-68 shrink-0 flex-col items-center px-2"
          >
            <span
              v-if="enabledRoles.length > 1"
              class="absolute top-0 border-t border-dashed border-rune-text-3/50"
              :class="
                index === 0
                  ? 'left-1/2 right-0'
                  : index === enabledRoles.length - 1
                    ? 'left-0 right-1/2'
                    : 'left-0 right-0'
              "
              aria-hidden="true"
            ></span>
            <span
              class="h-12 shrink-0 border-l border-dashed border-rune-text-3/50"
              aria-hidden="true"
            ></span>
            <PlaygroundFlowNode
              :step="String(index + 3).padStart(2, '0')"
              kind="Operation"
              :title="role.label"
              :status="roleStatus(role.id)"
              :icon="role.id === 'depth' ? 'depth' : 'scan'"
              tone="violet"
              :active="results[role.id].status === 'running'"
              input
              output
              :data-testid="'visual-node-' + role.id"
            >
              <div class="mt-4 rounded-rune-tile border border-white/5 bg-black/20 p-3">
                <p class="m-0 text-rune-caption text-rune-text-3">Model</p>
                <p class="mt-1 mb-0 break-words text-rune-body">
                  {{ roles[role.id].model || 'Select a model' }}
                </p>
              </div>
              <div
                v-if="plannerQuestions && role.id !== 'depth'"
                class="mt-3 rounded-rune-tile bg-rune-surface p-3"
              >
                <p class="m-0 text-rune-caption text-rune-text-3">Question</p>
                <p class="mt-1 mb-0 break-words text-rune-caption text-rune-text-2">
                  {{ plannerQuestions[role.id] }}
                </p>
              </div>
              <p
                v-if="results[role.id].error"
                role="alert"
                class="mb-0 break-words text-rune-caption text-error"
              >
                {{ results[role.id].error }}
              </p>
            </PlaygroundFlowNode>
            <span
              class="min-h-10 flex-1 border-l border-dashed border-rune-text-3/50"
              aria-hidden="true"
            ></span>
            <span
              v-if="enabledRoles.length > 1"
              class="absolute bottom-0 border-t border-dashed border-rune-text-3/50"
              :class="
                index === 0
                  ? 'left-1/2 right-0'
                  : index === enabledRoles.length - 1
                    ? 'left-0 right-1/2'
                    : 'left-0 right-0'
              "
              aria-hidden="true"
            ></span>
          </div>
        </div>
      </div>
      <PlaygroundFlowConnection label="Results" :tone="enabledRoles.length ? 'violet' : 'sky'" />
      <PlaygroundFlowNode
        :step="String(enabledRoles.length + 3).padStart(2, '0')"
        kind="Output"
        title="Analysis results"
        :status="outputStatus"
        icon="captions"
        tone="emerald"
        input
        class="max-w-[320px]"
        data-testid="visual-node-output"
        ><p class="mb-0 text-rune-caption text-rune-text-3">
          {{
            lastConfiguration
              ? 'Switch to App to view each result.'
              : 'Run the Flow to see results.'
          }}
        </p></PlaygroundFlowNode
      >
    </template>
    <template #app>
      <div
        v-if="!lastConfiguration && !running"
        class="my-auto flex min-h-64 flex-col items-center justify-center gap-2 text-center"
      >
        <p class="m-0 text-rune-body text-rune-text-2">Your analysis will appear here</p>
        <p class="m-0 text-rune-caption text-rune-text-3">
          Choose an input and models, then run the Flow.
        </p>
      </div>
      <div v-if="runRoles.length" class="flex w-full flex-col gap-4" data-testid="visual-results">
        <div>
          <p class="m-0 text-rune-heading text-rune-text-3 uppercase">Analysis results</p>
          <p class="mt-1 mb-0 text-rune-caption text-rune-text-2">
            Choose a branch to inspect its result.
          </p>
        </div>
        <div role="tablist" aria-label="Analysis branches" class="flex gap-2 overflow-x-auto pb-1">
          <RuneButton
            v-for="role in definitions.filter((item) => runRoles.includes(item.id))"
            :key="role.id"
            role="tab"
            :aria-label="role.label"
            :aria-selected="selectedResult === role.id"
            :tabindex="selectedResult === role.id ? 0 : -1"
            aria-controls="visual-result-panel"
            :variant="selectedResult === role.id ? 'primary' : 'secondary'"
            size="sm"
            class="shrink-0 normal-case!"
            @click="selectedResult = role.id"
            @keydown="navigateResults"
            >{{ role.label }}
            <span class="text-rune-caption opacity-70">{{ roleStatus(role.id) }}</span></RuneButton
          >
        </div>
        <section
          role="tabpanel"
          id="visual-result-panel"
          class="w-full rounded-rune-tile border border-white/10 bg-rune-sunken p-4"
          :aria-label="definitions.find((role) => role.id === selectedResult)?.label + ' result'"
        >
          <header class="flex items-center justify-between gap-3">
            <h2 class="m-0 text-rune-body">
              {{ definitions.find((role) => role.id === selectedResult)?.label }}
            </h2>
            <span class="text-rune-caption text-rune-text-3">{{ roleStatus(selectedResult) }}</span>
          </header>
          <div
            v-if="plannerQuestions && selectedResult !== 'depth'"
            class="mt-4 rounded-rune-tile bg-rune-surface p-3"
          >
            <p class="m-0 text-rune-heading text-rune-text-3 uppercase">Question</p>
            <p class="mt-1 mb-0 break-words text-rune-body">
              {{ plannerQuestions[selectedResult] }}
            </p>
          </div>
          <p
            v-if="results[selectedResult].status === 'waiting'"
            class="text-rune-caption text-rune-text-3"
          >
            Waiting for the planning step.
          </p>
          <p
            v-else-if="results[selectedResult].status === 'running'"
            class="text-rune-caption text-rune-text-3"
          >
            Analyzing this branch…
          </p>
          <p v-if="results[selectedResult].error" role="alert" class="break-words text-error">
            {{ results[selectedResult].error }}
          </p>
          <PlaygroundResult
            v-if="results[selectedResult].snapshot"
            :key="selectedResult"
            :snapshot="results[selectedResult].snapshot!"
            :live="false"
          />
          <PlaygroundDepth
            v-if="results[selectedResult].depth"
            :state="depthState(results[selectedResult])"
          />
        </section>
      </div>
    </template>
  </PlaygroundFlowLayout>
</template>