<script setup lang="ts">
defineProps<{ label: string; disabled?: boolean }>()

const model = defineModel<boolean>({ required: true })
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="model"
    :aria-label="label"
    :disabled="disabled"
    class="flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full p-1 transition-colors disabled:cursor-default disabled:opacity-40"
    :class="model ? 'bg-rune-ink' : 'bg-rune-raised'"
    @click="model = !model"
  >
    <span
      class="size-4 rounded-full transition-transform"
      :class="model ? 'translate-x-5 bg-black' : 'translate-x-0 bg-rune-text-3'"
    />
  </button>
</template>