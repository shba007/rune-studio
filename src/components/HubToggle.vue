<script setup lang="ts">
import HubRow from './HubRow.vue'

defineProps<{ label: string; hint?: string }>()

const model = defineModel<boolean>({ required: true })
</script>

<template>
  <HubRow :label="label" :hint="hint">
    <button
      type="button"
      role="switch"
      :aria-checked="model"
      :aria-label="label"
      class="flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full p-1 transition-colors"
      :class="model ? 'bg-rune-ink' : 'bg-rune-raised'"
      @click="model = !model"
    >
      <span
        class="size-4 rounded-full transition-transform"
        :class="model ? 'translate-x-5 bg-black' : 'translate-x-0 bg-rune-text-3'"
      />
    </button>
  </HubRow>
</template>