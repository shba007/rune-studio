<script setup lang="ts">
import type { IconName } from '@/icons'

import AppIcon from './AppIcon.vue'

defineProps<{
  label: string
  icon: IconName
  type?: 'text' | 'password'
  placeholder?: string
}>()

const model = defineModel<string>({ required: true })
</script>

<template>
  <label class="flex cursor-text items-center gap-2 rounded-2xl px-2 py-1 focus-within:bg-white/6">
    <span class="grid size-8 shrink-0 place-items-center rounded-full bg-white/8 text-zinc-300">
      <AppIcon :name="icon" />
    </span>
    <span class="flex min-w-0 flex-1 flex-col">
      <span class="text-label text-zinc-500">{{ label }}</span>
      <input
        v-model="model"
        :type="type ?? 'text'"
        :placeholder="placeholder"
        spellcheck="false"
        autocomplete="off"
        class="w-full min-w-0 bg-transparent text-body text-zinc-100 outline-none placeholder:text-zinc-600"
      />
    </span>
  </label>
</template>