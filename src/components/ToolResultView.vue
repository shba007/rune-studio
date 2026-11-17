<script setup lang="ts">
import { computed } from 'vue'
import { resultContent, schemaView, type ResultValue } from '@/config/toolResult'
import ToolResultValue from './ToolResultValue.vue'
import SchemaResultValue from './SchemaResultValue.vue'
const props = withDefaults(
  defineProps<{
    text: string
    schema?: unknown
    structuredContent?: ResultValue
    compact?: boolean
    showRaw?: boolean
  }>(),
  { structuredContent: undefined, showRaw: true },
)
const content = computed(() => resultContent(props.text))
const view = computed(() =>
  props.schema !== undefined && props.structuredContent !== undefined
    ? schemaView(props.schema, props.structuredContent)
    : null,
)
</script>

<template>
  <div
    class="flex min-w-0 flex-col text-rune-ink"
    :class="compact ? 'compact-result max-h-64 gap-2 overflow-y-auto' : 'gap-4'"
  >
    <SchemaResultValue v-if="view" :node="view" />
    <template v-else>
      <ToolResultValue v-for="(value, index) in content" :key="index" :value="value" />
    </template>
    <!-- The test bench keeps the exact text one click away; the pill shows the view only. -->
    <details v-if="!compact && showRaw" class="text-rune-caption text-rune-text-2">
      <summary class="cursor-pointer">Raw JSON</summary>
      <pre
        class="mt-3 max-h-96 overflow-auto rounded-rune-tile bg-rune-sunken p-4 text-rune-body whitespace-pre-wrap break-words"
        >{{ text }}</pre>
    </details>
  </div>
</template>

<style scoped>
/* Compact: the surrounding card is already the container, so the view draws no box of its
   own and uses the card's caption size throughout. */
.compact-result :deep(dl),
.compact-result :deep(div:has(> table)) {
  padding: 0;
  border-radius: 0;
  background: none;
}
.compact-result :deep(dl) {
  gap: 0.375rem;
}
/* Label beside value, so a small result fits without scrolling. */
.compact-result :deep(dl > div) {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
  column-gap: 0.5rem;
  padding: 0;
  border-width: 0;
}
/* A value that is itself a group, list or table gets the full width under its label. */
.compact-result :deep(dl > div:has(> dd > dl)),
.compact-result :deep(dl > div:has(> dd ol)),
.compact-result :deep(dl > div:has(> dd table)) {
  display: block;
}
.compact-result :deep(dd.pl-4) {
  margin-top: 0.25rem;
  padding-left: 0.5rem;
  border-left: 1px solid rgb(255 255 255 / 0.12);
}
.compact-result :deep(dt),
.compact-result :deep(th) {
  color: var(--color-rune-text-3);
}
.compact-result :deep(dt),
.compact-result :deep(dd),
.compact-result :deep(p),
.compact-result :deep(pre),
.compact-result :deep(table),
.compact-result :deep(th),
.compact-result :deep(td) {
  font-size: var(--text-rune-caption);
  line-height: var(--text-rune-caption--line-height);
}
.compact-result :deep(dd),
.compact-result :deep(td),
.compact-result :deep(p) {
  overflow-wrap: anywhere;
}
.compact-result :deep(table) {
  table-layout: fixed;
}
.compact-result :deep(th),
.compact-result :deep(td) {
  min-width: 0;
  padding: 0.25rem 0.375rem 0.25rem 0;
}
.compact-result :deep(ol) {
  gap: 0.25rem;
}
</style>