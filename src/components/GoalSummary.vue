<script setup lang="ts">
import { computed } from 'vue'
import type { IntakeEntry } from '@/types/decisions'
import AppIcon from './AppIcon.vue'
const props = defineProps<{ entry: IntakeEntry }>()
const goal = computed(() => props.entry.goal ?? props.entry.approvedGoal)
const reviewing = computed(() => !!props.entry.question && !!props.entry.approvedGoal)
const status = computed(() =>
  props.entry.status === 'pathway_planning'
    ? 'Comparing routes'
    : props.entry.status === 'task_planning'
      ? 'Drafting first-week tasks'
      : reviewing.value
        ? 'Review changes'
        : props.entry.approvedGoal
          ? 'Approved goal'
          : 'Defining your goal',
)
</script>
<template>
  <header class="flex flex-col gap-6" aria-label="Goal summary">
    <div>
      <div class="mb-rune-2 flex flex-wrap items-center gap-rune-2 text-rune-caption text-zinc-500">
        <span v-if="entry.category" class="uppercase tracking-wider">{{ entry.category }}</span>
        <span class="inline-flex items-center gap-1.5 text-zinc-300"
          ><AppIcon :name="entry.approvedGoal && !reviewing ? 'ok' : 'tune'" size="sm" />{{
            status
          }}</span
        >
        <span
          v-if="goal?.horizon"
          class="ml-auto inline-flex items-center gap-1.5 rounded-full bg-white/5 px-rune-2 py-1.5 text-zinc-300"
          ><AppIcon name="wait" size="sm" />{{ goal.horizon }}</span
        >
      </div>
      <h2 class="m-0 max-w-3xl font-sans text-rune-title font-medium text-zinc-100">
        {{ goal?.outcome || entry.text }}
      </h2>
    </div>
    <div v-if="goal" class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_24px_minmax(0,1fr)]">
      <div class="min-w-0 border-l-2 border-zinc-700 pl-4">
        <p class="m-0 mb-2 text-rune-caption uppercase tracking-wider text-zinc-500">
          Starting point
        </p>
        <p class="m-0 text-rune-body leading-relaxed text-zinc-300">
          {{ goal.current || 'Not supplied yet' }}
        </p>
      </div>
      <div class="hidden items-center justify-center text-zinc-600 sm:flex" aria-hidden="true">
        →
      </div>
      <div class="min-w-0 border-l-2 border-zinc-300 pl-4">
        <p class="m-0 mb-2 text-rune-caption uppercase tracking-wider text-zinc-500">
          Success looks like
        </p>
        <p class="m-0 text-rune-body leading-relaxed text-zinc-100">
          {{ goal.success || 'Not defined yet' }}
        </p>
      </div>
    </div>
    <dl v-if="goal" class="m-0 grid gap-4 text-rune-body sm:grid-cols-2">
      <div class="flex items-start gap-rune-2">
        <AppIcon name="fast" size="sm" class="mt-0.5 shrink-0 text-zinc-500" />
        <div>
          <dt class="text-rune-caption text-zinc-500">Available resources</dt>
          <dd class="m-0 mt-1 leading-relaxed text-zinc-300">
            {{ goal.resources || 'Not supplied yet' }}
          </dd>
        </div>
      </div>
      <div class="flex items-start gap-rune-2">
        <AppIcon name="shield" size="sm" class="mt-0.5 shrink-0 text-zinc-500" />
        <div>
          <dt class="text-rune-caption text-zinc-500">Constraints</dt>
          <dd class="m-0 mt-1 leading-relaxed text-zinc-300">
            {{ goal.constraints || 'Not supplied yet' }}
          </dd>
        </div>
      </div>
    </dl>
    <details class="text-rune-body text-zinc-500">
      <summary class="w-fit cursor-pointer py-1 hover:text-zinc-200">
        Original request{{ reviewing ? ' & approved definition' : '' }}
      </summary>
      <p class="mb-0 leading-relaxed whitespace-pre-wrap text-zinc-400">{{ entry.text }}</p>
      <dl v-if="reviewing && entry.approvedGoal" class="mt-4 grid gap-rune-2 sm:grid-cols-2">
        <div v-for="(value, key) in entry.approvedGoal" :key="key">
          <dt class="text-rune-caption capitalize text-zinc-500">{{ key }}</dt>
          <dd class="m-0 mt-1 text-zinc-300">{{ value }}</dd>
        </div>
      </dl>
    </details>
  </header>
</template>