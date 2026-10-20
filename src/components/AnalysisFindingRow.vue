<script setup lang="ts">
import { computed } from 'vue'

import type { IconName } from '@/icons'
import type { Finding, FindingKind } from '@/types/protocol'

import AppIcon from './AppIcon.vue'

/**
 * One finding in the accordion: a numbered badge, the label, what kind of thing it is and a chevron.
 * Opening it shows the description and the evidence as plain text (never HTML) and picks its marker on
 * the screen. A finding with no usable box has a dashed badge: it is listed, but nothing is drawn.
 */
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

const positioned = computed(() => props.finding.box !== null)
const badge = computed(() => {
  if (props.selected) return 'bg-[#f1f1ef] text-[#141414] border-[#f1f1ef]'
  if (!positioned.value) return 'border-dashed border-white/40 text-zinc-300'
  return 'border-white/50 text-zinc-100'
})
</script>

<template>
  <li :id="`finding-${finding.n}`" class="rounded-[20px] border border-white/10 bg-[#141414]">
    <button
      type="button"
      class="flex h-10 w-full cursor-pointer items-center gap-2 rounded-[20px] px-3 text-left outline-none focus-visible:ring-2 focus-visible:ring-white/60"
      :aria-expanded="open"
      :aria-label="`Finding ${finding.n}: ${finding.label}${positioned ? '' : ', no position on the screen'}`"
      @click="emit('toggle', finding.n)"
    >
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full border text-caption"
        :class="badge"
        :title="positioned ? '' : 'No position on the screen'"
      >
        {{ finding.n }}
      </span>
      <span class="min-w-0 flex-1 truncate text-status text-zinc-100">{{ finding.label }}</span>
      <AppIcon :name="KIND_ICON[finding.kind]" size="sm" class="shrink-0 text-zinc-500" />
      <AppIcon
        name="chevron"
        size="sm"
        class="shrink-0 text-zinc-400 motion-safe:transition-transform"
        :class="open ? 'rotate-180' : ''"
      />
    </button>
    <div v-if="open" class="flex flex-col gap-1 px-3 pb-3">
      <p v-if="finding.description" class="m-0 text-label text-zinc-200">
        {{ finding.description }}
      </p>
      <p class="m-0 text-label text-zinc-400">
        <span class="text-caption uppercase">Evidence</span>
        {{ finding.evidence || 'none given' }}
      </p>
      <p v-if="!positioned" class="m-0 text-caption text-zinc-500 uppercase">No position</p>
    </div>
  </li>
</template>