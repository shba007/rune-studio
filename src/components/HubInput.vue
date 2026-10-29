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
      class="h-10 w-full min-w-0 max-w-full sm:w-72 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3"
      @change="$emit('commit')"
      @keydown.enter="$emit('commit')"
    />
  </HubRow>
</template>