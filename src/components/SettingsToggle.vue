<script setup lang="ts">
import type { IconName } from '@/icons'

import AppIcon from './AppIcon.vue'

defineProps<{ label: string; icon: IconName }>()

const model = defineModel<boolean>({ required: true })
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="model"
    class="flex w-full cursor-pointer items-center gap-2 px-2 py-1 text-left"
    @click="model = !model"
  >
    <span class="grid size-8 shrink-0 place-items-center rounded-full bg-white/8 text-zinc-300">
      <AppIcon :name="icon" />
    </span>
    <span class="min-w-0 flex-1 py-1.5 text-body leading-snug text-zinc-100">{{ label }}</span>
    <span
      class="flex h-6 w-10 shrink-0 items-center rounded-full p-1 transition-colors"
      :class="model ? 'bg-[#f1f1ef]' : 'bg-white/15'"
    >
      <span
        class="size-4 rounded-full transition-transform"
        :class="model ? 'translate-x-4 bg-[#111]' : 'translate-x-0 bg-zinc-400'"
      />
    </span>
  </button>
</template>