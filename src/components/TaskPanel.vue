<script setup lang="ts">
import { useIntervalFn, useNow } from '@vueuse/core'
import { computed } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { clockText, remainingMs } from '@/config/eta'
import { KIND_ICON, TOOL_LABEL, kindOf } from '@/config/actions'

import AppIcon from './AppIcon.vue'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const goals = computed(() => agent.plan.value?.goals ?? [])
const current = computed(() => agent.plan.value?.current ?? 0)
const finished = computed(() => agent.plan.value?.finished ?? false)
const done = computed(() =>
  finished.value ? goals.value.length : Math.min(current.value, goals.value.length),
)

const now = useNow({ scheduler: (update) => useIntervalFn(update, 1000) })
const eta = computed(() => {
  if (!agent.isRunning.value || !agent.timing.value) return null
  const left = remainingMs(agent.timing.value, goals.value.length, done.value, now.value.getTime())
  if (left === null) return goals.value.length > 1 && done.value === 0 ? 'estimating…' : null
  return left <= 0 ? 'finishing…' : `${clockText(left)} left`
})
const goalElapsed = computed(() => {
  const timing = agent.timing.value
  return agent.isRunning.value && timing
    ? clockText(now.value.getTime() - timing.goalStartedAt)
    : null
})

const last = computed(() => agent.steps.value.at(-1) ?? null)
const lastLabel = computed(() =>
  last.value ? (TOOL_LABEL[last.value.tool] ?? last.value.tool) : '',
)
const lastDetail = computed(() => {
  const detail = last.value?.detail ?? ''
  return detail.length > 24 ? `${detail.slice(0, 23)}…` : detail
})

const message = computed(() => {
  const thinking = agent.thinking.value
  if (thinking && thinking.text && !thinking.done)
    return {
      label: thinking.model ? `${thinking.model} · requested` : 'Rune',
      text: thinking.text,
      live: true,
    }
  const response = agent.response.value
  return response
    ? {
        label: agent.settings.value?.externalAgent
          ? 'External agent'
          : agent.settings.value?.models?.agent
            ? agent.settings.value.models.agent
            : 'Rune',
        text: response,
        live: false,
      }
    : null
})

const modelCaveat = computed(() =>
  message.value?.label === agent.settings.value?.models?.agent
    ? 'Configured model; serving model not confirmed'
    : undefined,
)

function statusOf(index: number): 'done' | 'current' | 'pending' {
  if (finished.value || index < current.value) return 'done'
  return index === current.value ? 'current' : 'pending'
}
</script>

<template>
  <section
    class="flex w-full flex-col gap-2 rounded-rune-panel border border-rune-outline bg-rune-surface p-rune-2 font-rune-reading text-zinc-100"
  >
    <header class="flex h-6 items-center gap-2 whitespace-nowrap">
      <span class="text-rune-caption text-zinc-500 uppercase">Task</span>
      <span v-if="goals.length" class="text-rune-caption text-zinc-300 uppercase tabular-nums">
        {{ done }}/{{ goals.length }}
      </span>
      <span v-if="agent.steps.value.length" class="text-rune-caption text-zinc-500 uppercase">
        · {{ agent.steps.value.length }} {{ agent.steps.value.length === 1 ? 'step' : 'steps' }}
      </span>
      <span
        v-if="eta"
        class="ml-auto text-rune-caption text-(--accent) uppercase tabular-nums"
        data-testid="eta"
        >{{ eta }}</span
      >
      <button
        v-if="!agent.isRunning.value"
        type="button"
        class="ml-auto grid size-6 cursor-pointer place-items-center rounded-full text-zinc-400 transition hover:bg-white/8 hover:text-white"
        aria-label="Close the progress panel"
        @click="agent.clearTask()"
      >
        <AppIcon name="close" size="sm" />
      </button>
    </header>

    <p v-if="!goals.length" class="flex h-6 items-center gap-2 text-rune-caption text-zinc-400">
      <span class="planning size-2 rounded-full bg-primary" />
      Planning
    </p>

    <ol v-else class="flex flex-col">
      <li
        v-for="(goal, index) in goals"
        :key="index"
        class="flex h-6 items-center gap-2"
        :aria-current="statusOf(index) === 'current' ? 'step' : undefined"
      >
        <span class="grid size-4 shrink-0 place-items-center">
          <AppIcon v-if="statusOf(index) === 'done'" name="ok" class="text-primary" />
          <span
            v-else-if="statusOf(index) === 'current'"
            class="planning size-3 rounded-full bg-primary ring-2 ring-primary/30"
            :class="{ 'bg-attention! ring-attention/30!': agent.state.value === 'failed' }"
          />
          <span v-else class="size-3 rounded-full border border-white/25" />
        </span>
        <span
          class="min-w-0 flex-1 truncate text-rune-caption"
          :class="
            statusOf(index) === 'current'
              ? 'text-zinc-100'
              : statusOf(index) === 'done'
                ? 'text-zinc-500'
                : 'text-zinc-400'
          "
          :title="goal"
        >
          {{ goal }}
        </span>
        <span
          v-if="statusOf(index) === 'current' && goalElapsed"
          class="shrink-0 text-rune-caption text-zinc-400 tabular-nums"
          data-testid="goal-elapsed"
          >{{ goalElapsed }}</span
        >
      </li>
    </ol>

    <div
      v-if="last && agent.isRunning.value"
      class="flex h-8 items-center gap-2 rounded-full bg-white/5 px-3 text-rune-caption text-zinc-200"
    >
      <AppIcon :name="KIND_ICON[kindOf(last.tool)]" size="sm" class="shrink-0" />
      <span class="text-rune-caption whitespace-nowrap uppercase">{{ lastLabel }}</span>
      <span v-if="lastDetail" class="min-w-0 truncate text-zinc-400">{{ lastDetail }}</span>
    </div>

    <div v-if="message" class="flex flex-col gap-1">
      <span
        :title="modelCaveat"
        :aria-description="modelCaveat"
        class="break-all text-rune-caption text-zinc-500 uppercase"
        >{{ message.label }}</span
      >
      <p
        class="overflow-y-auto text-rune-caption whitespace-pre-wrap text-zinc-300"
        :class="agent.isRunning.value ? 'max-h-[72px]' : 'max-h-44'"
      >
        {{ message.text }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.planning {
  animation: planning-pulse 1.4s ease-in-out infinite;
}

@keyframes planning-pulse {
  50% {
    opacity: 0.4;
  }
}

@media (prefers-reduced-motion: reduce) {
  .planning {
    animation: none;
  }
}
</style>