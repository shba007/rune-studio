<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed, ref } from 'vue'
import AppIcon from './AppIcon.vue'

const props = defineProps<{
  refreshing?: boolean
  options: { target: { kind: string; id: number }; label: string }[]
}>()
const emit = defineEmits<{ refresh: [] }>()
function refresh() {
  expanded.value = false
  search.value = ''
  emit('refresh')
}
const selected = defineModel<string>({ required: true })
const expanded = ref(false)
const search = ref('')
const label = computed(
  () => props.options.find((o) => JSON.stringify(o.target) === selected.value)?.label,
)
const filtered = computed(() =>
  props.options.filter((o) => o.label.toLowerCase().includes(search.value.toLowerCase())),
)
function choose(value: string) {
  selected.value = value
  expanded.value = false
  search.value = ''
}
</script>

<template>
  <div class="min-w-0 rounded-rune-tile bg-rune-surface p-1" @keydown.esc.stop="expanded = false">
    <button
      type="button"
      aria-label="Choose capture target"
      :aria-expanded="expanded"
      class="flex min-h-11 w-full items-center gap-2 rounded-rune-tile px-2 text-left text-rune-caption text-rune-ink hover:bg-white/5"
      @click="expanded = !expanded"
    >
      <AppIcon :name="label?.startsWith('Screen') ? 'monitor' : 'app'" />
      <span class="min-w-0 flex-1 truncate">{{ label || 'Choose a screen or window' }}</span>
      <AppIcon name="chevron" size="sm" />
    </button>
    <RuneButton
      variant="ghost"
      size="sm"
      aria-label="Refresh capture targets"
      :disabled="refreshing"
      @click="refresh"
    >
      {{ refreshing ? 'Refreshing…' : 'Refresh targets' }}
    </RuneButton>
    <div v-if="expanded" class="flex flex-col gap-2 p-1">
      <input
        v-model="search"
        aria-label="Filter capture targets"
        placeholder="Find a screen or window"
        class="h-9 w-full rounded-rune-tile bg-black/20 px-2 text-rune-caption text-rune-ink"
      />
      <div class="max-h-44 overflow-x-hidden overflow-y-auto" aria-label="Capture targets">
        <template v-for="kind in ['screen', 'window']" :key="kind">
          <p
            v-if="filtered.some((o) => o.target.kind === kind)"
            class="px-2 py-1 text-rune-caption text-rune-text-2 uppercase"
          >
            {{ kind === 'screen' ? 'Screens' : 'Windows' }}
          </p>
          <button
            v-for="option in filtered.filter((o) => o.target.kind === kind)"
            :key="JSON.stringify(option.target)"
            type="button"
            :aria-pressed="selected === JSON.stringify(option.target)"
            class="flex w-full items-center gap-2 rounded-rune-tile px-2 py-2 text-left text-rune-caption text-rune-text-2 hover:bg-white/8"
            @click="choose(JSON.stringify(option.target))"
          >
            <span
              class="size-1.5 shrink-0 rounded-full"
              :class="selected === JSON.stringify(option.target) ? 'bg-white' : 'bg-rune-text-3'"
            />
            <span class="min-w-0 [overflow-wrap:anywhere]">{{ option.label }}</span>
          </button>
        </template>
        <p v-if="!filtered.length" class="px-2 py-3 text-rune-caption text-rune-text-2">
          No matching targets. Refresh the list to try again.
        </p>
      </div>
    </div>
  </div>
</template>