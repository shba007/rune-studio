<script setup lang="ts">
import type { SchemaView } from '@/config/toolResult'
import ToolResultValue from './ToolResultValue.vue'
defineProps<{ node: SchemaView }>()
</script>

<template>
  <pre v-if="node.kind === 'json'" class="m-0 text-rune-body whitespace-pre-wrap break-words">{{
    JSON.stringify(node.value, null, 2)
  }}</pre>
  <ToolResultValue v-else-if="node.kind === 'value'" :value="node.value" />
  <span v-else-if="node.kind === 'missing'" class="text-rune-text-3">—</span>
  <dl
    v-else-if="node.kind === 'object'"
    class="m-0 flex flex-col divide-y divide-white/10 rounded-rune-tile bg-rune-sunken px-4"
  >
    <div v-for="entry in node.fields" :key="entry.key" class="flex min-w-0 flex-col gap-2 py-3">
      <dt class="text-rune-caption text-rune-text-2" :title="entry.hint">{{ entry.label }}</dt>
      <dd class="m-0 min-w-0" :class="entry.node.kind === 'object' ? 'pl-4' : ''">
        <SchemaResultValue :node="entry.node" />
      </dd>
    </div>
  </dl>
  <div v-else-if="node.kind === 'list'">
    <p v-if="!node.items.length" class="m-0 text-rune-text-2">No items</p>
    <ol v-else class="m-0 flex list-decimal flex-col gap-3 pl-5">
      <li v-for="(item, index) in node.items" :key="index"><SchemaResultValue :node="item" /></li>
    </ol>
  </div>
  <div v-else class="max-w-full overflow-x-auto rounded-rune-tile bg-rune-sunken">
    <table class="w-full border-collapse text-left text-rune-body">
      <thead>
        <tr>
          <th
            v-for="column in node.columns"
            :key="column.key"
            :title="column.hint"
            scope="col"
            class="border-b border-white/10 px-4 py-3 text-rune-caption text-rune-text-2"
          >
            {{ column.label }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(row, index) in node.rows"
          :key="index"
          class="border-b border-white/10 last:border-0"
        >
          <td
            v-for="(cell, column) in row"
            :key="column"
            class="max-w-96 min-w-28 px-4 py-3 align-top"
          >
            <SchemaResultValue :node="cell" />
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>