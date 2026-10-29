<script setup lang="ts">
import { computed } from 'vue'

import type { IconName } from '@/icons'
import type { Finding, FindingKind } from '@/types/protocol'

import AppIcon from './AppIcon.vue'

const props = defineProps<{ finding: Finding; open: boolean; selected: boolean }>()
const emit = defineEmits<{ toggle: [n: number] }>()

const KIND_ICON: Record<FindingKind, IconName> = {
  text: 'keyboard',
  control: 'mouse',
  image: 'video',
  region: 'monitor',
  issue: 'stop',
  other: 'eye',
}

const positioned = computed(
  () => props.finding.box !== null || Boolean(props.finding.pose?.keypoints.length),
)
const badge = computed(() => {
  if (props.selected) return 'bg-rune-ink text-black'
  if (!positioned.value) return 'bg-rune-surface text-rune-text-3'
  return 'bg-rune-raised text-rune-ink'
})
</script>

<template>
  <li :id="`finding-${finding.n}`" class="rounded-rune-control bg-rune-surface">
    <button
      type="button"
      class="flex h-10 w-full cursor-pointer items-center gap-2 rounded-rune-control px-rune-1 text-left"
      :aria-expanded="open"
      :aria-label="`Finding ${finding.n}: ${finding.label}${positioned ? '' : ', no position on the screen'}`"
      @click="emit('toggle', finding.n)"
    >
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full text-rune-caption"
        :class="badge"
        :title="positioned ? '' : 'No position on the screen'"
      >
        {{ finding.n }}
      </span>
      <span class="min-w-0 flex-1 truncate text-rune-body text-rune-ink">{{ finding.label }}</span>
      <AppIcon :name="KIND_ICON[finding.kind]" size="sm" class="shrink-0 text-rune-text-2" />
      <AppIcon
        name="chevron"
        size="sm"
        class="shrink-0 text-rune-text-2 motion-safe:transition-transform"
        :class="open ? 'rotate-180' : ''"
      />
    </button>
    <p
      v-if="finding.pose?.rejected"
      role="status"
      class="m-0 px-3 pb-2 text-rune-caption text-amber-300"
    >
      Some pose data was invalid and omitted. Valid landmarks and connections are shown.
    </p>
    <div v-if="open" class="flex flex-col gap-1 px-3 pb-3">
      <p v-if="finding.description" class="m-0 text-rune-caption text-rune-ink">
        {{ finding.description }}
      </p>
      <p class="m-0 text-rune-caption text-rune-text-2">
        <span class="text-rune-caption uppercase">Evidence</span>
        {{ finding.evidence || 'none given' }}
      </p>
      <p v-if="!positioned" class="m-0 text-rune-caption text-rune-text-2 uppercase">No position</p>
    </div>
  </li>
</template>