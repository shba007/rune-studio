<script setup lang="ts">
import { onClickOutside } from '@vueuse/core'
import { computed, ref, useTemplateRef } from 'vue'
import AppIcon from './AppIcon.vue'
import RuneIconButton from './RuneIconButton.vue'

const props = defineProps<{
  refreshing?: boolean
  disabled?: boolean
  contained?: boolean
  inlineMenu?: boolean
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
const root = useTemplateRef<HTMLElement>('root')
onClickOutside(root, () => (expanded.value = false))
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
  <div
    ref="root"
    class="relative flex min-w-0 items-center gap-2"
    :class="[expanded && 'z-20', inlineMenu && 'flex-wrap']"
    @keydown.esc.stop="expanded = false"
  >
    <button
      type="button"
      aria-label="Choose capture target"
      :aria-expanded="expanded"
      :disabled="disabled"
      class="inline-flex h-8 max-w-64 min-w-0 cursor-pointer items-center gap-2 rounded-full bg-rune-sunken pr-3 pl-3 text-left text-rune-body text-rune-ink transition-colors hover:bg-black"
      :class="inlineMenu && 'flex-1'"
      :title="label || 'Choose a screen or window'"
      @click="expanded = !expanded"
    >
      <AppIcon :name="label?.startsWith('Screen') ? 'monitor' : 'app'" size="sm" />
      <span class="min-w-0 flex-1 truncate">{{ label || 'Choose a screen or window' }}</span>
      <AppIcon name="chevron" size="sm" class="text-rune-text-3" />
    </button>
    <RuneIconButton
      variant="ghost"
      size="sm"
      label="Refresh capture targets"
      :disabled="refreshing || disabled"
      @click="refresh"
    >
      <AppIcon name="refresh" size="sm" />
    </RuneIconButton>
    <div
      v-if="expanded && !disabled"
      class="flex flex-col gap-2 rounded-rune-tile bg-rune-raised p-2"
      :class="
        inlineMenu
          ? 'w-full min-w-0'
          : ['absolute top-full mt-1 max-w-[80vw]', contained ? 'left-0 w-full' : 'right-0 w-80']
      "
    >
      <input
        v-model="search"
        aria-label="Filter capture targets"
        placeholder="Find a screen or window"
        class="h-9 w-full rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink placeholder:text-rune-text-3"
      />
      <div class="max-h-48 overflow-x-hidden overflow-y-auto" aria-label="Capture targets">
        <template v-for="kind in ['screen', 'window']" :key="kind">
          <p
            v-if="filtered.some((o) => o.target.kind === kind)"
            class="m-0 px-3 py-1 font-sans text-rune-heading text-rune-text-3 uppercase"
          >
            {{ kind === 'screen' ? 'Screens' : 'Windows' }}
          </p>
          <button
            v-for="option in filtered.filter((o) => o.target.kind === kind)"
            :key="JSON.stringify(option.target)"
            type="button"
            :aria-pressed="selected === JSON.stringify(option.target)"
            class="flex w-full min-w-0 cursor-pointer items-center gap-2 rounded-rune-control px-3 py-2 text-left text-rune-body transition-colors hover:bg-white/8"
            :title="option.label"
            :class="
              selected === JSON.stringify(option.target) ? 'text-rune-ink' : 'text-rune-text-2'
            "
            @click="choose(JSON.stringify(option.target))"
          >
            <span
              class="size-1.5 shrink-0 rounded-full"
              :class="selected === JSON.stringify(option.target) ? 'bg-white' : 'bg-rune-text-3'"
            />
            <span class="min-w-0 flex-1 truncate">{{ option.label }}</span>
          </button>
        </template>
        <p v-if="!filtered.length" class="m-0 px-3 py-3 text-rune-caption text-rune-text-3">
          No matching targets. Refresh the list to try again.
        </p>
      </div>
    </div>
  </div>
</template>