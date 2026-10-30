<script setup lang="ts">
import { computed, ref } from 'vue'
import { DOMAINS, UNASSIGNED, domainIcon, domainOf, type Domain } from '@/config/domains'
import { goalStatus, goalTitle } from '@/config/goalStatus'
import { outcomeProgress } from '@/config/workspaceProgress'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
import AppIcon from './AppIcon.vue'
import RuneChoice from './RuneChoice.vue'
import RuneButton from './RuneButton.vue'
import RuneDot from './RuneDot.vue'
import RuneMenu from './RuneMenu.vue'

const props = defineProps<{
  entries: IntakeEntry[]
  tasks: WorkspaceTask[]
  selectedId?: string | null
}>()
const emit = defineEmits<{ open: [id: string]; remove: [entry: IntakeEntry] }>()

const { showSanctuary, overrides } = useDomainPrefs()
const FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Active' },
  { value: 'question', label: 'Needs answer' },
]
const filter = ref('all')
const confirming = ref<string | null>(null)

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
const taskCount = (entry: IntakeEntry) =>
  props.tasks.filter((task) => task.parentGoalId === entry.id).length
const progress = (entry: IntakeEntry) => outcomeProgress(entry.outcomeMetric)
const horizon = (entry: IntakeEntry) => (entry.approvedGoal ?? entry.goal)?.horizon ?? '—'
const cols =
  'grid grid-cols-[minmax(0,1fr)_9rem_2.5rem] items-center gap-rune-2 @2xl:grid-cols-[minmax(0,1fr)_9rem_6rem_4rem_7rem_2.5rem]'
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
      <span />
    </div>
    <section
      v-for="group in groups"
      :key="group.domain.id"
      class="flex flex-col gap-0.5"
      :data-domain="group.domain.id"
    >
      <h3
        class="m-0 flex items-center gap-2 px-rune-2 py-1 font-sans text-rune-body font-normal text-rune-ink"
      >
        <AppIcon :name="domainIcon(group.domain.id)" size="sm" class="text-rune-text-2" />
        <span>{{ group.domain.name }}</span>
        <span class="text-rune-caption text-rune-text-3">{{ group.rows.length }}</span>
      </h3>
      <p v-if="!group.rows.length" class="m-0 px-rune-3 py-1 text-rune-caption text-rune-text-3">
        No goals in this domain yet.
      </p>
      <template v-for="entry in group.rows" :key="entry.id">
        <div
          :data-goal="entry.id"
          class="cursor-pointer bg-rune-surface px-rune-2 py-rune-1 transition-colors first:rounded-t-rune-tile last:rounded-b-rune-tile hover:bg-rune-raised"
          :class="[cols, selectedId === entry.id && 'bg-rune-raised']"
          @click="$emit('open', entry.id)"
        >
          <button
            type="button"
            class="line-clamp-2 cursor-pointer text-left text-rune-body text-rune-ink"
            @click.stop="$emit('open', entry.id)"
          >
            {{ goalTitle(entry) }}
          </button>
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
          <span class="justify-self-end" @click.stop>
            <RuneMenu
              :label="`More actions: ${goalTitle(entry)}`"
              :items="[{ id: 'delete', label: 'Delete goal', danger: true }]"
              @select="confirming = entry.id"
            />
          </span>
        </div>
        <div
          v-if="confirming === entry.id"
          role="alertdialog"
          :aria-label="`Delete goal: ${goalTitle(entry)}`"
          class="flex flex-wrap items-center justify-between gap-rune-2 rounded-rune-tile bg-rune-raised px-rune-2 py-rune-1 text-rune-body text-rune-text-2"
        >
          <span>Delete this goal? This cannot be undone.</span>
          <span class="flex gap-rune-1">
            <RuneButton variant="ghost" size="sm" @click="confirming = null">Cancel</RuneButton>
            <RuneButton
              variant="danger"
              size="sm"
              @click="((confirming = null), emit('remove', entry))"
            >
              Delete
            </RuneButton>
          </span>
        </div>
      </template>
    </section>
    <p v-if="hiddenCount" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
      {{ hiddenCount }} goal{{ hiddenCount === 1 ? '' : 's' }} in Sanctuary domains
      {{ hiddenCount === 1 ? 'is' : 'are' }} hidden. Turn them on in Settings.
    </p>
  </section>
</template>