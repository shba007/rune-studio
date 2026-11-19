<script setup lang="ts">
import type { HabitsModel } from '@/composables/useHabits'

import RuneButton from './RuneButton.vue'

defineProps<{ habits: HabitsModel }>()
</script>

<template>
  <section
    aria-label="Today's habits"
    class="flex flex-col gap-rune-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-body text-rune-ink"
  >
    <header class="flex items-center justify-between">
      <span class="text-rune-caption text-rune-text-2 uppercase">Today</span>
    </header>
    <ul class="m-0 flex list-none flex-wrap gap-2 p-0">
      <li v-for="chip in habits.chips.value" :key="chip.id">
        <RuneButton
          size="sm"
          :variant="chip.done ? 'ghost' : 'secondary'"
          :disabled="chip.done || habits.pending.value"
          :aria-pressed="chip.done"
          :aria-label="
            chip.done
              ? `${chip.title}: done today`
              : chip.kind === 'timed'
                ? `Start focus for ${chip.title}`
                : `Check in ${chip.title}`
          "
          :class="chip.done ? 'line-through' : ''"
          @click="chip.kind === 'timed' ? habits.startFocus(chip.id) : habits.checkIn(chip.id)"
        >
          <span aria-hidden="true">{{ chip.done ? '✓' : chip.kind === 'timed' ? '▶' : '○' }}</span>
          {{ chip.label }}
        </RuneButton>
      </li>
    </ul>
    <p v-if="habits.error.value" role="alert" class="m-0 text-rune-caption text-error">
      {{ habits.error.value }}
    </p>
  </section>
</template>