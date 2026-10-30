<script setup lang="ts">
import { computed, ref, useId } from 'vue'
import { DOMAINS, UNASSIGNED, domainIcon, domainOf, type Domain } from '@/config/domains'
import { goalTitle } from '@/config/goalStatus'
import { outcomeProgress, planProgress, taskProgress } from '@/config/workspaceProgress'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import type { IconName } from '@/icons'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneIconButton from './RuneIconButton.vue'

const props = defineProps<{ entries: IntakeEntry[]; tasks: WorkspaceTask[] }>()
const emit = defineEmits<{ openGoal: [id: string]; openTask: [id: string] }>()
const { showSanctuary, overrides } = useDomainPrefs()
const pattern = useId()

const domainId = ref<string | null>(null)
const goalId = ref<string | null>(null)
const origin = ref('50% 50%')
const direction = ref<'in' | 'out'>('in')

interface Spot {
  id: string
  label: string
  icon: IconName
  percent: number | null
  note: string
  x: number
  y: number
  measured: boolean
}

const goals = computed(() => props.entries.filter((entry) => entry.kind === 'goal'))
const goalsOf = (id: string) => goals.value.filter((e) => domainOf(e, overrides.value).id === id)
const tasksOf = (id: string) => props.tasks.filter((task) => task.parentGoalId === id)
const goalPercent = (entry: IntakeEntry) =>
  outcomeProgress(entry.outcomeMetric) ?? planProgress(tasksOf(entry.id)).value
const average = (values: (number | null)[]) => {
  const known = values.filter((v): v is number => v !== null)
  return known.length ? Math.round(known.reduce((sum, v) => sum + v, 0) / known.length) : null
}
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`

function ring(count: number, make: (index: number) => Omit<Spot, 'x' | 'y'>): Spot[] {
  return Array.from({ length: count }, (_, index) => {
    const angle = -Math.PI / 2 + (index / count) * Math.PI * 2
    return { ...make(index), x: 50 + 36 * Math.cos(angle), y: 50 + 34 * Math.sin(angle) }
  })
}

const domains = computed<Domain[]>(() => {
  const shown = DOMAINS.filter((domain) => showSanctuary.value || domain.zone === 'engine')
  return goalsOf(UNASSIGNED.id).length ? [...shown, UNASSIGNED] : shown
})
const domain = computed(() => domains.value.find((item) => item.id === domainId.value) ?? null)
const goal = computed(() => goals.value.find((item) => item.id === goalId.value) ?? null)
const level = computed<'life' | 'domain' | 'goal'>(() =>
  goal.value ? 'goal' : domain.value ? 'domain' : 'life',
)

const spots = computed<Spot[]>(() => {
  if (level.value === 'life')
    return ring(domains.value.length, (index) => {
      const item = domains.value[index]!
      const own = goalsOf(item.id)
      const measured = item.zone === 'engine'
      return {
        id: item.id,
        label: item.name,
        icon: domainIcon(item.id),
        measured,
        percent: measured ? average(own.map(goalPercent)) : null,
        note: plural(own.length, 'goal'),
      }
    })
  if (level.value === 'domain') {
    const own = goalsOf(domain.value!.id)
    const measured = domain.value!.zone === 'engine'
    return ring(own.length, (index) => {
      const entry = own[index]!
      return {
        id: entry.id,
        label: goalTitle(entry),
        icon: 'target',
        measured,
        percent: measured ? goalPercent(entry) : null,
        note: plural(tasksOf(entry.id).length, 'task'),
      }
    })
  }
  const own = tasksOf(goal.value!.id)
  const measured = domain.value?.zone !== 'sanctuary'
  return ring(own.length, (index) => {
    const task = own[index]!
    return {
      id: task.id,
      label: task.title,
      icon: 'history',
      measured,
      percent: measured ? taskProgress(task) : null,
      note: '',
    }
  })
})

const hub = computed(() => {
  if (level.value === 'life') return { label: 'Life', icon: 'logo' as IconName, empty: '' }
  if (level.value === 'domain')
    return {
      label: domain.value!.name,
      icon: domainIcon(domain.value!.id),
      empty: 'No goals in this domain yet.',
    }
  return { label: goalTitle(goal.value!), icon: 'target' as IconName, empty: 'No tasks yet.' }
})
const crumbs = computed(() => [
  { key: 'life', label: 'Life' },
  ...(domain.value ? [{ key: 'domain', label: domain.value.name }] : []),
  ...(goal.value ? [{ key: 'goal', label: goalTitle(goal.value) }] : []),
])

function enter(spot: Spot) {
  origin.value = `${spot.x}% ${spot.y}%`
  direction.value = 'in'
  if (level.value === 'life') domainId.value = spot.id
  else if (level.value === 'domain') goalId.value = spot.id
  else emit('openTask', spot.id)
}
function up(to: 'life' | 'domain') {
  origin.value = '50% 50%'
  direction.value = 'out'
  if (to === 'life') domainId.value = null
  goalId.value = null
}
const back = () => up(level.value === 'goal' ? 'domain' : 'life')
const label = (spot: Spot) =>
  `${spot.label}${spot.percent !== null ? `, ${spot.percent}%` : ''}${spot.note ? `, ${spot.note}` : ''}`
const dash = (percent: number | null) => `${percent ?? 0} 100`
</script>

<template>
  <section aria-label="Life graph" class="@container relative min-h-0 flex-1">
    <nav
      aria-label="Life path"
      class="absolute top-3 right-3 left-3 z-10 flex min-h-8 items-center gap-1 text-rune-body"
    >
      <RuneIconButton
        v-if="level !== 'life'"
        variant="ghost"
        size="sm"
        label="Zoom out"
        @click="back"
      >
        <AppIcon name="back" size="sm" />
      </RuneIconButton>
      <template v-for="(crumb, index) in crumbs" :key="crumb.key">
        <span v-if="index" class="text-rune-text-3" aria-hidden="true">›</span>
        <RuneButton
          v-if="index < crumbs.length - 1"
          variant="ghost"
          size="sm"
          @click="up(crumb.key as 'life' | 'domain')"
        >
          {{ crumb.label }}
        </RuneButton>
        <span v-else class="truncate px-2 text-rune-ink">{{ crumb.label }}</span>
      </template>
      <RuneButton
        v-if="level === 'goal'"
        variant="ghost"
        size="sm"
        class="ml-auto"
        @click="emit('openGoal', goalId!)"
      >
        Open goal →
      </RuneButton>
    </nav>
    <div class="absolute inset-0 overflow-hidden bg-rune-sunken">
      <svg class="absolute inset-0 size-full" aria-hidden="true">
        <defs>
          <pattern :id="pattern" width="24" height="24" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="1" fill="white" opacity="0.22" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" :fill="`url(#${pattern})`" />
      </svg>
      <Transition :name="`zoom-${direction}`" mode="out-in">
        <div
          :key="`${domainId}:${goalId}`"
          class="absolute inset-0"
          :style="{ transformOrigin: origin }"
          :data-level="level"
        >
          <svg
            class="absolute inset-0 size-full text-rune-text-3"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <line
              v-for="spot in spots"
              :key="spot.id"
              x1="50"
              y1="50"
              :x2="spot.x"
              :y2="spot.y"
              stroke="currentColor"
              stroke-width="1"
              stroke-dasharray="3 4"
              vector-effect="non-scaling-stroke"
              opacity="0.5"
            />
          </svg>
          <div
            class="absolute top-1/2 left-1/2 flex w-40 -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1 text-center"
            data-testid="hub"
          >
            <span class="grid size-14 place-items-center rounded-full bg-rune-raised text-rune-ink">
              <AppIcon :name="hub.icon" />
            </span>
            <span class="line-clamp-2 text-rune-body text-rune-ink">{{ hub.label }}</span>
            <span v-if="!spots.length" class="text-rune-caption text-rune-text-3">{{
              hub.empty
            }}</span>
          </div>
          <button
            v-for="spot in spots"
            :key="spot.id"
            type="button"
            :data-node="spot.id"
            :aria-label="label(spot)"
            class="group absolute flex w-28 -translate-x-1/2 -translate-y-1/2 cursor-pointer flex-col items-center gap-1 text-center"
            :style="{ left: `${spot.x}%`, top: `${spot.y}%` }"
            @click="enter(spot)"
          >
            <span
              class="relative grid size-14 place-items-center rounded-full bg-rune-surface text-rune-ink transition-colors group-hover:bg-rune-raised"
            >
              <svg
                v-if="spot.measured"
                viewBox="0 0 36 36"
                class="absolute inset-0 size-full -rotate-90"
                aria-hidden="true"
              >
                <circle
                  cx="18"
                  cy="18"
                  r="16"
                  fill="none"
                  stroke-width="2"
                  class="stroke-rune-sunken"
                />
                <circle
                  cx="18"
                  cy="18"
                  r="16"
                  fill="none"
                  stroke-width="2"
                  stroke-linecap="round"
                  pathLength="100"
                  :stroke-dasharray="dash(spot.percent)"
                  class="stroke-rune-ink"
                />
              </svg>
              <AppIcon :name="spot.icon" />
            </span>
            <span class="line-clamp-2 text-rune-caption text-rune-ink">{{ spot.label }}</span>
            <span class="text-rune-caption text-rune-text-3 tabular-nums">
              {{ spot.percent !== null ? `${spot.percent}%` : spot.note }}
            </span>
          </button>
        </div>
      </Transition>
    </div>
  </section>
</template>

<style scoped>
@media (prefers-reduced-motion: no-preference) {
  .zoom-in-enter-active,
  .zoom-in-leave-active,
  .zoom-out-enter-active,
  .zoom-out-leave-active {
    transition:
      transform 0.28s ease,
      opacity 0.28s ease;
  }
  .zoom-in-leave-to,
  .zoom-out-enter-from {
    transform: scale(2.2);
    opacity: 0;
  }
  .zoom-in-enter-from,
  .zoom-out-leave-to {
    transform: scale(0.5);
    opacity: 0;
  }
}
</style>