<script setup lang="ts">
import { computed } from 'vue'
import { goalStatus, goalTitle } from '@/config/goalStatus'
import type { IntakeEntry } from '@/types/decisions'
import RuneChip from './RuneChip.vue'
const props = defineProps<{ entry: IntakeEntry }>()
const goal = computed(() => props.entry.goal ?? props.entry.approvedGoal)
const reviewing = computed(() => !!props.entry.question && !!props.entry.approvedGoal)
const status = computed(() => goalStatus(props.entry).label)
const facts = computed(() =>
  goal.value
    ? [
        { label: 'Starting point', value: goal.value.current || 'Not supplied yet' },
        { label: 'Success looks like', value: goal.value.success || 'Not defined yet' },
        { label: 'Available resources', value: goal.value.resources || 'Not supplied yet' },
        { label: 'Constraints', value: goal.value.constraints || 'Not supplied yet' },
      ]
    : [],
)
</script>
<template>
  <header class="flex flex-col gap-rune-2" aria-label="Goal summary">
    <div class="flex flex-wrap items-center gap-rune-1">
      <RuneChip v-if="entry.category">{{ entry.category }}</RuneChip>
      <RuneChip :variant="entry.approvedGoal && !reviewing ? 'ink' : 'neutral'">{{
        status
      }}</RuneChip>
      <RuneChip v-if="goal?.horizon">{{ goal.horizon }}</RuneChip>
    </div>
    <h2 class="m-0 max-w-3xl font-sans text-rune-title font-medium text-rune-ink">
      {{ goalTitle(entry) }}
    </h2>
    <div v-if="facts.length" class="flex flex-col gap-0.5">
      <div
        v-for="fact in facts"
        :key="fact.label"
        class="flex flex-col gap-1 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
      >
        <span class="text-rune-heading text-rune-text-3 uppercase">{{ fact.label }}</span>
        <span class="text-rune-body text-rune-ink">{{ fact.value }}</span>
      </div>
    </div>
    <details class="text-rune-body text-rune-text-2">
      <summary class="w-fit cursor-pointer px-rune-2 py-1 hover:text-rune-ink">
        Original request{{ reviewing ? ' & approved definition' : '' }}
      </summary>
      <p class="mx-rune-2 mt-rune-1 mb-0 whitespace-pre-wrap text-rune-text-2">{{ entry.text }}</p>
      <dl
        v-if="reviewing && entry.approvedGoal"
        class="mx-rune-2 mt-rune-2 grid gap-rune-2 sm:grid-cols-2"
      >
        <div v-for="(value, key) in entry.approvedGoal" :key="key">
          <dt class="text-rune-heading text-rune-text-3 uppercase">{{ key }}</dt>
          <dd class="m-0 mt-1 text-rune-text-2">{{ value }}</dd>
        </div>
      </dl>
    </details>
  </header>
</template>