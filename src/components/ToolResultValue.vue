<script setup lang="ts">
import { computed } from 'vue'
import { resultLabel, type ResultValue } from '@/config/toolResult'
const props = withDefaults(defineProps<{ value: ResultValue; depth?: number }>(), { depth: 0 })
const record = (value: ResultValue): value is Record<string, ResultValue> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)
const rows = computed(() =>
  Array.isArray(props.value) && props.value.length && props.value.every(record)
    ? (props.value as Record<string, ResultValue>[])
    : null,
)
const columns = computed(() => [...new Set(rows.value?.flatMap((row) => Object.keys(row)) ?? [])])
</script>

<template>
  <pre v-if="depth >= 12" class="m-0 whitespace-pre-wrap break-words">{{
    JSON.stringify(value, null, 2)
  }}</pre>
  <div
    v-else-if="rows && columns.length"
    class="max-w-full overflow-x-auto rounded-rune-tile bg-rune-sunken"
  >
    <table class="w-full border-collapse text-left text-rune-body">
      <thead>
        <tr>
          <th
            v-for="column in columns"
            :key="column"
            scope="col"
            class="border-b border-white/10 px-4 py-3 text-rune-caption text-rune-text-2"
          >
            {{ resultLabel(column) }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(row, index) in rows"
          :key="index"
          class="border-b border-white/10 last:border-0"
        >
          <td v-for="column in columns" :key="column" class="max-w-96 min-w-28 px-4 py-3 align-top">
            <ToolResultValue
              v-if="Object.hasOwn(row, column)"
              :value="row[column]!"
              :depth="depth + 1"
            />
            <span v-else class="text-rune-text-3">—</span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
  <div v-else-if="Array.isArray(value)">
    <p v-if="!value.length" class="m-0 text-rune-text-2">No items</p>
    <ol v-else class="m-0 flex list-decimal flex-col gap-3 pl-5">
      <li v-for="(item, index) in value" :key="index">
        <ToolResultValue :value="item" :depth="depth + 1" />
      </li>
    </ol>
  </div>
  <div v-else-if="record(value)">
    <p v-if="!Object.keys(value).length" class="m-0 text-rune-text-2">No fields</p>
    <dl
      v-else
      class="m-0 flex flex-col divide-y divide-white/10 rounded-rune-tile bg-rune-sunken px-4"
    >
      <div v-for="(item, key) in value" :key="key" class="flex min-w-0 flex-col gap-2 py-3">
        <dt class="text-rune-caption text-rune-text-2" :title="String(key)">
          {{ resultLabel(String(key)) }}
        </dt>
        <dd class="m-0 min-w-0"><ToolResultValue :value="item" :depth="depth + 1" /></dd>
      </div>
    </dl>
  </div>
  <p
    v-else
    class="m-0 text-rune-body whitespace-pre-wrap"
    :class="typeof value === 'string' && !/\s/.test(value) ? 'break-all' : 'break-words'"
  >
    {{
      value === null
        ? 'Null'
        : value === true
          ? 'Yes'
          : value === false
            ? 'No'
            : value === ''
              ? '(empty)'
              : value
    }}
  </p>
</template>