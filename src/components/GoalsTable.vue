<script setup lang="ts">
import { computed, ref } from 'vue'
import { DOMAINS, UNASSIGNED, domainOf, type Domain } from '@/config/domains'
import { goalStatus, goalTitle } from '@/config/goalStatus'
import { outcomeProgress } from '@/config/workspaceProgress'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
import AppIcon from './AppIcon.vue'
import RuneChoice from './RuneChoice.vue'
import RuneDot from './RuneDot.vue'

const props = defineProps<{
  entries: IntakeEntry[]
  tasks: WorkspaceTask[]
  selectedId?: string | null
}>()
defineEmits<{ open: [id: string] }>()

const { showSanctuary, overrides } = useDomainPrefs()
const FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Active' },
  { value: 'question', label: 'Needs answer' },
]
const filter = ref('all')
const collapsed = ref<Record<string, boolean>>({})

const goals = computed(() =>
  props.entries.filter((entry) => {
    if (entry.kind !== 'goal') return false
    if (filter.value === 'question') return !!entry.question
    if (filter.value === 'active') return !!entry.approvedGoal && !entry.question
    return true
  }),
)
const visibleDomains = computed<Domain[]>(() => [
  ...DOMAINS.filter((domain) => showSanctuary.value || domain.zone === 'engine'),
  UNASSIGNED,
])
const groups = computed(() =>
  visibleDomains.value
    .map((domain) => ({
      domain,
      rows: goals.value.filter((entry) => domainOf(entry, overrides.value).id === domain.id),
    }))
    .filter((group) => group.domain.id !== UNASSIGNED.id || group.rows.length),
)
const hiddenCount = computed(() =>
  showSanctuary.value
    ? 0
    : goals.value.filter((entry) => domainOf(entry, overrides.value).zone === 'sanctuary').length,
)
const isCollapsed = (group: { domain: Domain; rows: unknown[] }) =>
  collapsed.value[group.domain.id] ?? group.rows.length === 0
const toggle = (group: { domain: Domain; rows: unknown[] }) =>
  (collapsed.value = { ...collapsed.value, [group.domain.id]: !isCollapsed(group) })
const taskCount = (entry: IntakeEntry) =>
  props.tasks.filter((task) => task.parentGoalId === entry.id).length
const progress = (entry: IntakeEntry) => outcomeProgress(entry.outcomeMetric)
const horizon = (entry: IntakeEntry) => (entry.approvedGoal ?? entry.goal)?.horizon ?? '—'
const cols =
  'grid grid-cols-[minmax(0,1fr)_9rem] items-center gap-rune-2 @2xl:grid-cols-[minmax(0,1fr)_9rem_6rem_4rem_7rem]'
</script>

<template>
  <section aria-label="Goals table" class="@container flex flex-col gap-rune-2">
    <RuneChoice v-model="filter" label="Filter goals" segments :options="FILTERS" />
    <div
      :class="[cols, 'px-rune-2 text-rune-heading text-rune-text-3 uppercase']"
      aria-hidden="true"
    >
      <span>Goal</span>
      <span>Status</span>
      <span class="hidden @2xl:block">Horizon</span>
      <span class="hidden @2xl:block">Tasks</span>
      <span class="hidden @2xl:block">Progress</span>
    </div>
    <section
      v-for="group in groups"
      :key="group.domain.id"
      class="flex flex-col gap-0.5"
      :data-domain="group.domain.id"
    >
      <button
        type="button"
        class="flex cursor-pointer items-center gap-2 rounded-rune-tile px-rune-2 py-1 text-left text-rune-body text-rune-ink hover:bg-white/5"
        :aria-expanded="!isCollapsed(group)"
        @click="toggle(group)"
      >
        <AppIcon
          name="chevron"
          size="sm"
          class="text-rune-text-3 transition-transform"
          :class="isCollapsed(group) && '-rotate-90'"
        />
        <span>{{ group.domain.name }}</span>
        <span class="text-rune-caption text-rune-text-3">{{ group.rows.length }}</span>
      </button>
      <template v-if="!isCollapsed(group)">
        <p v-if="!group.rows.length" class="m-0 px-rune-3 py-1 text-rune-caption text-rune-text-3">
          No goals in this domain yet.
        </p>
        <button
          v-for="entry in group.rows"
          :key="entry.id"
          type="button"
          :data-goal="entry.id"
          class="cursor-pointer bg-rune-surface px-rune-2 py-rune-1 text-left transition-colors first:rounded-t-rune-tile last:rounded-b-rune-tile hover:bg-rune-raised"
          :class="[cols, selectedId === entry.id && 'bg-rune-raised']"
          @click="$emit('open', entry.id)"
        >
          <span class="line-clamp-2 text-rune-body text-rune-ink">{{ goalTitle(entry) }}</span>
          <span class="flex items-center gap-2 text-rune-caption text-rune-text-2">
            <RuneDot :state="goalStatus(entry).tone" />
            <span class="truncate">{{ goalStatus(entry).label }}</span>
          </span>
          <span class="hidden truncate text-rune-caption text-rune-text-2 @2xl:block">{{
            horizon(entry)
          }}</span>
          <span class="hidden text-rune-caption text-rune-text-2 tabular-nums @2xl:block">{{
            taskCount(entry)
          }}</span>
          <span class="hidden items-center gap-2 @2xl:flex">
            <span v-if="progress(entry) !== null" class="h-1.5 flex-1 rounded-full bg-rune-sunken">
              <span
                class="block h-full rounded-full bg-rune-ink"
                :style="{ width: `${progress(entry)}%` }"
              />
            </span>
            <span class="text-rune-caption text-rune-text-3 tabular-nums">{{
              progress(entry) === null ? '—' : `${progress(entry)}%`
            }}</span>
          </span>
        </button>
      </template>
    </section>
    <p v-if="hiddenCount" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
      {{ hiddenCount }} goal{{ hiddenCount === 1 ? '' : 's' }} in Sanctuary domains
      {{ hiddenCount === 1 ? 'is' : 'are' }} hidden. Turn them on in Settings.
    </p>
  </section>
</template>