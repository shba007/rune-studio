<script setup lang="ts">
import RuneDot from './RuneDot.vue'

withDefaults(
  defineProps<{
    title: string
    meta?: string
    state?: 'idle' | 'ok' | 'warn' | 'down' | 'busy'
    selected?: boolean
    interactive?: boolean
  }>(),
  { selected: false, interactive: false },
)
defineEmits<{ open: [] }>()
</script>

<template>
  <div
    class="flex min-w-0 flex-col gap-2 px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
    :class="selected ? 'bg-rune-raised' : 'bg-rune-surface'"
  >
    <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
      <slot name="leading" />
      <RuneDot v-if="state" :state="state" />
      <component
        :is="interactive ? 'button' : 'div'"
        :type="interactive ? 'button' : undefined"
        class="flex min-w-48 flex-1 flex-col gap-1 text-left"
        :class="interactive && 'cursor-pointer'"
        @click="interactive && $emit('open')"
      >
        <span class="line-clamp-2 text-rune-body text-rune-ink">{{ title }}</span>
        <span v-if="meta" class="text-rune-caption text-rune-text-3">{{ meta }}</span>
      </component>
      <div v-if="$slots.actions" class="flex flex-wrap items-center gap-rune-1">
        <slot name="actions" />
      </div>
    </div>
    <slot />
  </div>
</template>