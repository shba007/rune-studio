<script setup lang="ts">
import type { Finding } from '@/types/protocol'

/** The evidence column: one row per finding, linked both ways with the markers on the picture. */
defineProps<{ findings: Finding[]; active: number | null; showBoxes: boolean }>()
const emit = defineEmits<{ hover: [n: number | null]; select: [n: number] }>()
</script>

<template>
  <ol class="m-0 flex list-none flex-col gap-2 p-0" aria-label="Findings">
    <li v-for="finding in findings" :key="finding.n" :id="`finding-${finding.n}`">
      <button
        type="button"
        class="block w-full cursor-pointer rounded-lg border p-3 text-left outline-none motion-safe:transition focus-visible:ring-2 focus-visible:ring-current"
        :class="[
          active === finding.n
            ? 'border-current bg-black/6 dark:bg-white/10'
            : 'border-black/12 hover:bg-black/4 dark:border-white/12 dark:hover:bg-white/6',
          finding.kind === 'issue' ? 'border-l-[3px] border-l-[#d71921]' : '',
        ]"
        :aria-pressed="active === finding.n"
        @mouseenter="emit('hover', finding.n)"
        @mouseleave="emit('hover', null)"
        @focus="emit('hover', finding.n)"
        @blur="emit('hover', null)"
        @click="emit('select', finding.n)"
      >
        <span class="flex items-baseline gap-2 text-caption uppercase">
          <span class="opacity-60">{{ String(finding.n).padStart(2, '0') }}</span>
          <span class="min-w-0 flex-1 truncate">{{ finding.label }}</span>
          <span class="opacity-60">{{ finding.kind }}</span>
        </span>
        <span v-if="finding.description" class="mt-1 block text-body">{{
          finding.description
        }}</span>
        <span class="mt-2 block text-label opacity-70">
          <span class="text-caption uppercase">Evidence</span>
          {{ finding.evidence || 'none given' }}
        </span>
        <span v-if="showBoxes && !finding.box" class="mt-1 block text-caption uppercase opacity-60"
          >No location</span
        >
      </button>
    </li>
  </ol>
</template>