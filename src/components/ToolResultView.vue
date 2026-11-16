<script setup lang="ts">
import { computed } from 'vue'
import { resultContent } from '@/config/toolResult'
import ToolResultValue from './ToolResultValue.vue'
const props = defineProps<{ text: string }>()
const content = computed(() => resultContent(props.text))
</script>

<template>
  <div class="flex min-w-0 flex-col gap-4 text-rune-ink">
    <ToolResultValue v-for="(value, index) in content" :key="index" :value="value" />
    <details class="text-rune-caption text-rune-text-2">
      <summary class="cursor-pointer">Raw JSON</summary>
      <pre
        class="mt-3 max-h-96 overflow-auto rounded-rune-tile bg-rune-sunken p-4 text-rune-body whitespace-pre-wrap break-words"
        >{{ text }}</pre>
    </details>
  </div>
</template>