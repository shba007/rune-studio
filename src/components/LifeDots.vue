<script setup lang="ts">
import { computed } from 'vue'
import { DOMAINS, domainIcon, domainOf } from '@/config/domains'
import { outcomeProgress } from '@/config/workspaceProgress'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import type { IntakeEntry } from '@/types/decisions'
import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'

const props = defineProps<{ entries: IntakeEntry[] }>()
const emit = defineEmits<{ open: [] }>()
const { overrides, showSanctuary } = useDomainPrefs()

const dots = computed(() => {
  const goals = props.entries.filter((entry) => entry.kind === 'goal' && entry.approvedGoal)
  return DOMAINS.filter((domain) => showSanctuary.value || domain.zone === 'engine').map(
    (domain) => {
      const own = goals.filter((entry) => domainOf(entry, overrides.value).id === domain.id)
      const measured = own
        .map((entry) => outcomeProgress(entry.outcomeMetric))
        .filter((value): value is number => value !== null)
      const percent = measured.length
        ? Math.round(measured.reduce((sum, value) => sum + value, 0) / measured.length)
        : null
      const level = !own.length ? 0.14 : percent === null ? 0.4 : 0.4 + (0.6 * percent) / 100
      return { id: domain.id, name: domain.name, goals: own.length, percent, level }
    },
  )
})
const active = computed(() => dots.value.some((dot) => dot.goals > 0))
const weakest = computed(() => {
  const measured = dots.value.filter((dot) => dot.percent !== null)
  if (measured.length < 2) return null
  return measured.reduce((low, dot) => (dot.percent! < low.percent! ? dot : low)).id
})
const describe = (dot: (typeof dots.value)[number]) =>
  `${dot.name}: ${dot.percent !== null ? `${dot.percent}%` : dot.goals ? 'goal set' : 'no goal'}`
const summary = computed(() => `Life progress. ${dots.value.map(describe).join('. ')}. Open Goals.`)
</script>

<template>
  <div v-if="active" class="w-full rounded-b-rune-composer bg-rune-surface">
    <RuneButton
      variant="ghost"
      class="h-10 w-full justify-between rounded-b-rune-composer rounded-t-none px-rune-2"
      :aria-label="summary"
      data-testid="life-dots"
      @click="emit('open')"
    >
      <span
        v-for="dot in dots"
        :key="dot.id"
        :title="describe(dot)"
        :data-dot="dot.id"
        class="grid size-5 place-items-center text-rune-ink"
        :class="weakest === dot.id && 'motion-safe:animate-pulse'"
        :style="{ opacity: dot.level }"
      >
        <AppIcon :name="domainIcon(dot.id)" size="sm" />
      </span>
    </RuneButton>
  </div>
</template>