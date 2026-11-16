<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { resultContent, schemaView, type ResultValue } from '@/config/toolResult'
import ToolResultValue from './ToolResultValue.vue'
import SchemaResultValue from './SchemaResultValue.vue'
import RuneButton from './RuneButton.vue'
const props = withDefaults(
  defineProps<{ text: string; schema?: unknown; structuredContent?: ResultValue }>(),
  { structuredContent: undefined },
)
const content = computed(() => resultContent(props.text))
const view = computed(() =>
  props.schema !== undefined && props.structuredContent !== undefined
    ? schemaView(props.schema, props.structuredContent)
    : null,
)
const raw = ref(false)
watch(
  () => props.text,
  () => (raw.value = false),
)
</script>

<template>
  <div class="flex min-w-0 flex-col gap-4 text-rune-ink">
    <template v-if="view">
      <RuneButton variant="ghost" size="sm" class="self-end" :aria-pressed="raw" @click="raw = !raw"
        >Raw</RuneButton
      >
      <pre v-if="raw" class="m-0 text-rune-body whitespace-pre-wrap break-words">{{ text }}</pre>
      <SchemaResultValue v-else :node="view" />
    </template>
    <template v-else>
      <ToolResultValue v-for="(value, index) in content" :key="index" :value="value" />
      <details class="text-rune-caption text-rune-text-2">
        <summary class="cursor-pointer">Raw JSON</summary>
        <pre
          class="mt-3 max-h-96 overflow-auto rounded-rune-tile bg-rune-sunken p-4 text-rune-body whitespace-pre-wrap break-words"
          >{{ text }}</pre>
      </details>
    </template>
  </div>
</template>