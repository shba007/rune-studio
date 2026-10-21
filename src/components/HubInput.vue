<script setup lang="ts">
import HubRow from './HubRow.vue'

defineProps<{
  label: string
  hint?: string
  type?: 'text' | 'password'
  placeholder?: string
}>()
defineEmits<{ commit: [] }>()

const model = defineModel<string>({ required: true })
</script>

<template>
  <HubRow :label="label" :hint="hint">
    <slot name="before" />
    <input
      v-model="model"
      :type="type ?? 'text'"
      :placeholder="placeholder"
      :aria-label="label"
      spellcheck="false"
      autocomplete="off"
      class="h-9 w-full min-w-0 max-w-full sm:w-72 rounded-xl border-0 bg-white/6 px-3 text-body text-zinc-100 outline-none placeholder:text-zinc-600 focus-visible:ring-2 focus-visible:ring-white/40"
      @change="$emit('commit')"
      @keydown.enter="$emit('commit')"
    />
  </HubRow>
</template>