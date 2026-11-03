<script setup lang="ts">
import type { IconName } from '@/icons'
import AppIcon from './AppIcon.vue'

const props = defineProps<{
  step: string
  kind: string
  title: string
  status: string
  icon: IconName
  tone: 'sky' | 'violet' | 'emerald'
  active?: boolean
  input?: boolean
  output?: boolean
}>()
const colors = {
  sky: { icon: 'bg-sky-400/15 text-sky-300', active: 'border-sky-400/70' },
  violet: {
    icon: 'bg-violet-400/15 text-violet-300',
    active: 'border-violet-400/70',
  },
  emerald: {
    icon: 'bg-emerald-400/15 text-emerald-300',
    active: 'border-emerald-400/70',
  },
}
</script>

<template>
  <div class="relative w-full shrink-0">
    <span
      class="absolute -top-8 left-1 inline-flex max-w-full items-center gap-1 rounded-full px-2 py-0.5 text-rune-caption"
      :class="colors[props.tone].icon"
    >
      <AppIcon :name="icon" size="sm" /> <span>{{ step }} · {{ kind }}</span>
    </span>
    <article
      class="relative h-full rounded-rune-tile border bg-rune-raised p-3 shadow-lg"
      :class="active ? colors[props.tone].active : 'border-white/10'"
    >
      <span
        v-if="input"
        class="absolute -top-1.5 left-1/2 size-3 -translate-x-1/2 rounded-full border border-rune-text-3 bg-rune-surface"
        aria-hidden="true"
      ></span>
      <div>
        <h2 class="m-0 text-rune-body text-rune-ink">{{ title }}</h2>
        <span
          class="mt-2 inline-flex whitespace-nowrap rounded-full bg-white/5 px-2 py-0.5 text-rune-caption text-rune-text-2"
          >{{ status }}</span
        >
      </div>
      <slot />
      <span
        v-if="output"
        class="absolute -bottom-1.5 left-1/2 size-3 -translate-x-1/2 rounded-full border border-rune-text-3 bg-rune-surface"
        aria-hidden="true"
      ></span>
    </article>
  </div>
</template>