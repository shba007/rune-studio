<script setup lang="ts">
import { onKeyStroke, useElementSize } from '@vueuse/core'
import { computed, ref, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import { useUpdater } from '@/composables/useUpdater'
import { DOCK_GAP, DOCK_WIDTH, PAGE_GAP, PAGE_WIDTH } from '@/config/layout'

import Composer from './Composer.vue'
import OrbDock from './OrbDock.vue'
import ConfirmCard from './ConfirmCard.vue'
import { useAnalysis } from '@/composables/useAnalysis'
import { useExternalAgent } from '@/composables/useExternalAgent'

import AnalysisCard from './AnalysisCard.vue'
import GlyphOrb from './GlyphOrb.vue'
import ReflectionCard from './ReflectionCard.vue'
import ServerStatusCard from './ServerStatusCard.vue'
import FeedbackCard from './FeedbackCard.vue'
import ReviewCard from './ReviewCard.vue'
import HealthDots from './HealthDots.vue'
import ProgressRing from './ProgressRing.vue'
import TaskPanel from './TaskPanel.vue'
import TaskBoard from './TaskBoard.vue'
import { useTaskBoard } from '@/composables/useTaskBoard'
import { taskOrbState } from '@/config/taskActivity'
import { useDecisions } from '@/composables/useDecisions'
import DecisionInbox from './DecisionInbox.vue'
import PersonalitySymbol from './PersonalitySymbol.vue'
import { usePersonality } from '@/composables/usePersonality'
import TaskReminder from './TaskReminder.vue'
import ModelIcon from './ModelIcon.vue'
import { useModelActivity } from '@/composables/useModelActivity'
import { requestProgress, visibleGoal } from '@/config/taskProgress'
import { pillCards, type PillCard } from '@/config/pillCards'
import { orbHint } from '@/config/orbHint'

const agent = useAgent()
const modelActivity = useModelActivity()
const activeModels = modelActivity.models
const modelTooltip = computed(() =>
  modelActivity.active.value
    .map((item) => `${item.model} (${item.reported ? 'reported by server' : 'requested'})`)
    .join(', '),
)
const personality = usePersonality()
const taskBoard = useTaskBoard()
const decisions = useDecisions()
const expandedDecision = ref<string | null>(null)
const decisionKey = computed(() =>
  decisions.current.value
    ? `${decisions.current.value.id}:${decisions.current.value.revision}`
    : null,
)
const decisionInKnowledge = computed(
  () => !!decisionKey.value && expandedDecision.value === decisionKey.value,
)
watch(
  () =>
    decisions.current.value
      ? `${decisions.current.value.id}:${decisions.current.value.revision}`
      : undefined,
  (id) => {
    if (id && agent.mode.value === 'ambient') void agent.setMode('open')
  },
)
const workState = computed(() =>
  agent.settings.value?.externalAgent
    ? agent.state.value
    : decisions.busy.value && !agent.isRunning.value
      ? 'planning'
      : taskOrbState(agent.state.value, taskBoard.requests.value),
)
const boardWorking = computed(
  () =>
    decisions.busy.value ||
    taskBoard.requests.value.some((r) => ['queued', 'planning', 'running'].includes(r.phase)),
)
const updater = useUpdater()

const updating = updater.updating
const downloadPercent = computed(() =>
  updater.phase.value === 'downloading' && updater.progress.value !== null
    ? Math.round(updater.progress.value * 100)
    : null,
)
const promptVisible = computed(
  () =>
    updater.prompt.value &&
    updater.phase.value === 'available' &&
    !agent.isRunning.value &&
    !agent.question.value,
)
watch(promptVisible, (visible) => {
  if (visible && agent.mode.value === 'ambient') void agent.setMode('open')
})

const {
  orbAccent,
  darkAccent,
  mark: agentMark,
  logo: agentLogo,
  letter: agentLetter,
} = useExternalAgent()

const reflecting = computed(() => agent.state.value === 'reflecting' && agent.reflection.value)
const boardGoals = computed(() => {
  if (agent.settings.value?.externalAgent || !taskBoard.requests.value.length) return null
  const active = taskBoard.requests.value.filter((r) =>
    ['queued', 'planning', 'running'].includes(r.phase),
  )
  return (active.length ? active : taskBoard.requests.value.slice(-1)).flatMap((r) => r.goals)
})
const boardDone = computed(
  () =>
    boardGoals.value?.filter(
      (g) =>
        g.tasks.length &&
        g.tasks.every((id) => taskBoard.tasks.value.find((t) => t.id === id)?.phase === 'done'),
    ).length ?? 0,
)
const boardProgress = computed(() => {
  if (agent.settings.value?.externalAgent || reflecting.value) return null
  const request = visibleGoal(taskBoard.requests.value)
  if (!request) return null
  return requestProgress(taskBoard.tasks.value.filter((task) => task.requestId === request.id))
})
const ringGoals = computed(() =>
  reflecting.value
    ? reflecting.value.total
    : (boardGoals.value?.length ?? agent.plan.value?.goals.length ?? 0),
)
const ringCurrent = computed(() =>
  reflecting.value
    ? reflecting.value.done
    : boardGoals.value
      ? boardDone.value
      : (agent.plan.value?.current ?? 0),
)
const ringFinished = computed(() =>
  reflecting.value
    ? reflecting.value.done >= reflecting.value.total
    : boardGoals.value
      ? boardGoals.value.length > 0 && boardDone.value === boardGoals.value.length
      : (agent.plan.value?.finished ?? false),
)

const analysis = useAnalysis()
const analysisCard = useTemplateRef<InstanceType<typeof AnalysisCard>>('analysisCard')
const submitAnalysis = (text: string) => analysisCard.value?.submit(text) ?? Promise.resolve(false)
watch(
  () => analysis.snapshot.value.status,
  (status) => {
    if (status !== 'idle' && agent.mode.value !== 'analysis') void agent.setMode('analysis')
    else if (status === 'idle' && agent.mode.value === 'analysis') void agent.setMode('open')
  },
)

const isAmbient = computed(() => agent.mode.value === 'ambient')

const plan = computed(() =>
  pillCards({
    question: !!agent.question.value,
    analysis: agent.mode.value === 'analysis',
    decision: !!decisions.current.value,
    decisionInKnowledge: decisionInKnowledge.value,
    taskPanel:
      agent.mode.value === 'task' &&
      (!taskBoard.requests.value.length || !!agent.settings.value?.externalAgent),
    board: taskBoard.requests.value.length > 0,
    working: agent.isRunning.value || boardWorking.value,
    server: ['down', 'warn'].includes(agent.health.value.tone),
    reflection: !!agent.reflectionPrompt.value,
    review: !!agent.reviewOffer.value && agent.reviewStage.value === 'review',
    feedback: !!agent.reviewOffer.value && agent.reviewStage.value !== 'review',
    update: !!promptVisible.value && !!updater.latest.value,
  }),
)
const shows = (card: PillCard) => plan.value.cards.includes(card)

if (import.meta.env.DEV) Object.assign(window, { __demoAgent: agent })

const content = useTemplateRef<HTMLElement>('content')
const { height: contentHeight } = useElementSize(content)
watch(contentHeight, (height) => {
  const available = Math.min(1200, Math.max(200, window.screen.availHeight * 0.9))
  if (height > available && decisionKey.value && !decisionInKnowledge.value) {
    expandedDecision.value = decisionKey.value
    void agent.openWorkspace(decisions.current.value?.kind === 'goal' ? 'goals' : 'tasks')
    return
  }
  if (height >= 48) void agent.fitHeight(Math.ceil(height))
})

onKeyStroke('Escape', () => {
  if (agent.question.value || agent.mode.value === 'analysis') return
  void agent.dismiss()
})

const hint = computed(() =>
  orbHint({
    updating: updating.value,
    working: agent.isRunning.value || boardWorking.value,
    ambient: isAmbient.value,
  }),
)
const orbTitle = computed(() =>
  updating.value
    ? 'Updating Rune Studio'
    : activeModels.value.length && !agent.settings.value?.externalAgent
      ? `Active models: ${modelTooltip.value}`
      : agent.health.value.title,
)
const showPersonality = computed(
  () =>
    !agent.settings.value?.externalAgent && ['ok', 'checking'].includes(agent.health.value.tone),
)

function onOrb() {
  if (hint.value.action === 'busy') return
  if (hint.value.action === 'stop') void agent.stop()
  else void agent.toggleOpen()
}
</script>

<template>
  <div
    class="flex h-full w-full items-end justify-end"
    :style="{ '--accent': darkAccent ?? 'var(--color-voice)' }"
    @pointerdown="agent.noteActivity()"
    @keydown="agent.noteActivity()"
  >
    <div
      class="h-16 shrink-0 self-end overflow-hidden transition-[width] duration-300 ease-out motion-reduce:transition-none"
      :style="{ width: isAmbient ? '0px' : `${DOCK_WIDTH + DOCK_GAP}px` }"
    >
      <div class="flex h-full justify-start" :style="{ width: `${DOCK_WIDTH}px` }">
        <OrbDock :agent="agent" />
      </div>
    </div>
    <div
      data-tauri-drag-region
      :title="`Personality: ${personality.name}`"
      class="relative isolate flex size-16 shrink-0 items-center justify-center overflow-hidden rounded-full border-0 bg-rune-surface outline-none [clip-path:circle(50%)]"
    >
      <span
        v-if="showPersonality"
        class="pointer-events-none absolute bottom-1 left-1/2 z-20 grid size-4 -translate-x-1/2 place-items-center rounded-full bg-black/85 text-[10px] leading-none text-zinc-100"
        :aria-label="`Active personality: ${personality.name}`"
        ><PersonalitySymbol :mark="personality.mark" class="size-3"
      /></span>
      <svg v-if="updating" viewBox="0 0 64 64" class="update-ring" aria-hidden="true">
        <circle cx="32" cy="32" r="30" fill="none" class="update-track" />
        <circle
          v-if="downloadPercent !== null"
          cx="32"
          cy="32"
          r="30"
          fill="none"
          pathLength="100"
          class="update-fill"
          :stroke-dasharray="`${downloadPercent} 100`"
        />
        <circle
          v-else-if="updater.phase.value !== 'restarting'"
          cx="32"
          cy="32"
          r="30"
          fill="none"
          pathLength="100"
          class="update-fill update-sweep"
          stroke-dasharray="28 100"
        />
        <circle
          v-else
          cx="32"
          cy="32"
          r="30"
          fill="none"
          pathLength="100"
          class="update-fill"
          stroke-dasharray="100 100"
        />
      </svg>
      <ProgressRing
        v-else
        :goals="ringGoals"
        :progress="boardProgress"
        :current="ringCurrent"
        :finished="ringFinished"
        :running="agent.isRunning.value || boardWorking || agent.state.value === 'reflecting'"
        :failed="workState === 'failed'"
        :accent="darkAccent"
      />
      <button
        type="button"
        class="relative size-14 cursor-pointer border-0 rounded-full bg-primary focus-visible:-outline-offset-3"
        :aria-label="hint.label"
        :aria-description="orbTitle"
        :title="orbTitle"
        @click="onOrb"
      >
        <GlyphOrb
          :state="updating || analysis.busy.value ? 'planning' : workState"
          :level="agent.level.value"
          :accent="orbAccent"
          :mark="agentMark"
          :logo="agentLogo"
          :letter="agentLetter"
        />
        <span
          v-if="!agent.settings.value?.externalAgent && activeModels.length"
          role="img"
          class="pointer-events-none absolute inset-0 grid place-items-center"
          :aria-label="
            activeModels.length > 1 ? 'Multiple models active' : `Active model: ${activeModels[0]}`
          "
        >
          <span class="grid size-7 place-items-center rounded-full bg-primary text-[#111]">
            <span v-if="activeModels.length > 1" class="text-[11px] font-bold">{{
              activeModels.length
            }}</span>
            <ModelIcon v-else :model="activeModels[0]!" class="!size-5" />
          </span>
        </span>
        <HealthDots :tone="agent.health.value.tone" />
      </button>
    </div>

    <div
      class="h-full overflow-hidden transition-[width] duration-300 ease-out motion-reduce:transition-none"
      :style="{ width: isAmbient ? '0px' : `${PAGE_WIDTH + PAGE_GAP}px` }"
    >
      <div class="ml-2 flex h-full flex-col justify-end" :style="{ width: `${PAGE_WIDTH}px` }">
        <div ref="content" class="flex flex-col">
          <Transition name="page" mode="out-in">
            <div key="main" class="flex flex-col gap-2">
              <Transition name="page">
                <TaskPanel v-if="shows('taskPanel')" :agent="agent" />
              </Transition>
              <TaskBoard v-if="shows('board')" :board="taskBoard" />
              <TaskReminder v-if="shows('reminder')" :agent="agent" />
              <DecisionInbox v-if="shows('decision')" compact :model="decisions" />
              <button
                v-if="shows('decisionLink')"
                class="rounded-2xl border border-white/10 p-3 text-label text-zinc-200"
                @click="
                  agent.openWorkspace(decisions.current.value?.kind === 'goal' ? 'goals' : 'tasks')
                "
              >
                Continue your request in Workspace
              </button>
              <AnalysisCard
                v-if="shows('analysis')"
                ref="analysisCard"
                :analysis="analysis"
                @close="agent.mode.value === 'analysis' && agent.setMode('open')"
              />
              <section
                v-if="shows('update') && updater.latest.value"
                class="flex w-full flex-col gap-2 rounded-[28px] border border-white/15 bg-rune-surface p-4 text-zinc-100"
                role="alertdialog"
                aria-label="Update available"
              >
                <header class="flex h-6 items-center gap-2">
                  <span class="size-2 shrink-0 rounded-full bg-primary" />
                  <span class="text-caption text-zinc-300 uppercase">Update available</span>
                </header>
                <p class="text-body">
                  Version {{ updater.latest.value.version }}
                  <span class="text-zinc-500">(you have {{ updater.version.value }})</span>
                </p>
                <div class="mt-1 flex gap-2">
                  <button
                    type="button"
                    class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 text-label text-zinc-200 transition hover:bg-white/10"
                    @click="updater.dismissPrompt()"
                  >
                    Later
                  </button>
                  <button
                    type="button"
                    class="h-8 flex-1 cursor-pointer rounded-full bg-primary text-label text-[#111] transition hover:bg-white"
                    @click="updater.install(agent.isRunning.value)"
                  >
                    Update
                  </button>
                </div>
              </section>
              <ServerStatusCard v-if="shows('server')" :agent="agent" />
              <ReflectionCard v-if="shows('reflection')" :agent="agent" />
              <FeedbackCard v-if="shows('feedback')" :agent="agent" />
              <ReviewCard v-if="shows('review')" :agent="agent" />
              <p
                v-if="plan.more"
                role="status"
                class="m-0 px-rune-2 text-rune-caption text-zinc-500 uppercase"
              >
                {{ plan.more }} more waiting
              </p>
              <ConfirmCard
                v-if="shows('confirm') && agent.question.value"
                :key="agent.question.value.id"
                :agent="agent"
              />
              <Composer
                :agent="agent"
                :board="taskBoard"
                :intake-busy="decisions.busy.value"
                :analysis-submit="agent.mode.value === 'analysis' ? submitAnalysis : undefined"
                :analysis-busy="analysis.busy.value"
                :cancel-analysis="analysis.cancel"
              />
            </div>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.update-ring {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  transform: rotate(-90deg);
}

.update-track {
  stroke: rgb(241 241 239 / 0.18);
  stroke-width: 2;
}

.update-fill {
  stroke: var(--accent);
  stroke-width: 2.5;
  stroke-linecap: round;
  transition: stroke-dasharray 250ms ease-out;
}

.update-sweep {
  transform-origin: 32px 32px;
  animation: update-spin 1.1s linear infinite;
}

@keyframes update-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .update-sweep {
    animation: none;
  }
}
</style>