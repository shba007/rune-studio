<script setup lang="ts" generic="T extends string | number | boolean">
import { onClickOutside } from '@vueuse/core'
import { ref, useTemplateRef } from 'vue'

import type { IconName } from '@/icons'

import AppIcon from './AppIcon.vue'
import RuneSegment from './RuneSegment.vue'

const props = defineProps<{
  label: string
  options: { value: T; label: string; disabled?: boolean; icon?: IconName; iconClass?: string }[]
  disabled?: boolean
  menu?: boolean
  segments?: boolean
  icon?: IconName
}>()

const model = defineModel<T>({ required: true })

const open = ref(false)
const root = useTemplateRef<HTMLElement>('root')
onClickOutside(root, () => (open.value = false))

const asSegments = () => !props.menu && (props.segments || props.options.length <= 3)
const selectedOption = () => props.options.find((o) => o.value === model.value)
const selectedLabel = () => selectedOption()?.label ?? ''

function pick(value: T) {
  model.value = value
  open.value = false
}

function step(delta: number) {
  const index = props.options.findIndex((o) => o.value === model.value)
  const next = props.options[(index + delta + props.options.length) % props.options.length]
  if (next) model.value = next.value
}
</script>

<template>
  <div
    v-if="asSegments()"
    role="radiogroup"
    :aria-label="label"
    class="inline-flex gap-1 rounded-full bg-rune-sunken p-1"
  >
    <RuneSegment
      v-for="option in options"
      :key="String(option.value)"
      role="radio"
      :aria-checked="option.value === model"
      :selected="option.value === model"
      :disabled="disabled || option.disabled"
      @click="model = option.value"
    >
      {{ option.label }}
    </RuneSegment>
  </div>
  <div v-else ref="root" class="relative" :class="open && 'z-20'" @keydown.esc="open = false">
    <button
      type="button"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-label="label"
      :disabled="disabled"
      class="inline-flex h-8 cursor-pointer items-center gap-2 rounded-full bg-rune-sunken pr-3 pl-4 text-rune-body text-rune-ink transition-colors hover:bg-black disabled:cursor-default disabled:opacity-40"
      @click="open = !open"
      @keydown.down.prevent="step(1)"
      @keydown.up.prevent="step(-1)"
    >
      <AppIcon
        v-if="selectedOption()?.icon ?? icon"
        :name="(selectedOption()?.icon ?? icon)!"
        size="sm"
        :class="selectedOption()?.iconClass ?? 'text-rune-text-3'"
      />
      {{ selectedLabel() }}
      <AppIcon name="chevron" size="sm" class="text-rune-text-3" />
    </button>
    <ul
      v-if="open"
      role="listbox"
      :aria-label="label"
      class="glass absolute top-full right-0 mt-1 flex min-w-full flex-col gap-0.5 rounded-rune-tile bg-rune-raised p-1"
    >
      <li v-for="option in options" :key="String(option.value)" role="presentation">
        <button
          type="button"
          role="option"
          :disabled="option.disabled"
          :aria-selected="option.value === model"
          class="flex w-full cursor-pointer items-center justify-between gap-4 rounded-full px-3 py-2 text-left text-rune-body whitespace-nowrap transition-colors hover:bg-white/8 disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent"
          :class="option.value === model ? 'text-rune-ink' : 'text-rune-text-2'"
          @click="pick(option.value)"
        >
          <span class="flex items-center gap-2">
            <AppIcon
              v-if="option.icon"
              :name="option.icon"
              size="sm"
              :class="option.iconClass ?? 'text-rune-text-3'"
            />
            {{ option.label }}
          </span>
          <AppIcon v-if="option.value === model" name="check" size="sm" />
        </button>
      </li>
    </ul>
  </div>
</template>