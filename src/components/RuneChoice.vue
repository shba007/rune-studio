<script setup lang="ts" generic="T extends string | number | boolean">
import { onClickOutside, useElementBounding, useEventListener, useWindowSize } from '@vueuse/core'
import { computed, ref, useTemplateRef } from 'vue'

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
  floating?: boolean
}>()

const model = defineModel<T>({ required: true })

const open = ref(false)
const root = useTemplateRef<HTMLElement>('root')
const popup = useTemplateRef<HTMLElement>('popup')
const bounds = useElementBounding(root)
const viewport = useWindowSize()
onClickOutside(root, () => (open.value = false), { ignore: [popup] })
useEventListener(
  window,
  'scroll',
  () => {
    if (open.value && props.floating) bounds.update()
  },
  { capture: true, passive: true },
)
const floatingStyle = computed(() => {
  const width = Math.min(Math.max(bounds.width.value, 260), Math.max(0, viewport.width.value - 16))
  const left = Math.max(8, Math.min(bounds.left.value, viewport.width.value - width - 8))
  const below = viewport.height.value - bounds.bottom.value - 8
  const above = bounds.top.value - 8
  const upwards = below < 180 && above > below
  return {
    width: `${width}px`,
    left: `${left}px`,
    maxHeight: `${Math.max(0, Math.min(256, (upwards ? above : below) - 4))}px`,
    ...(upwards
      ? { bottom: `${viewport.height.value - bounds.top.value + 4}px` }
      : { top: `${bounds.bottom.value + 4}px` }),
  }
})

const asSegments = () => !props.menu && (props.segments || props.options.length <= 3)
const selectedOption = () => props.options.find((o) => o.value === model.value)
const selectedLabel = () => selectedOption()?.label ?? ''

function pick(value: T) {
  model.value = value
  open.value = false
}

function step(delta: number) {
  const available = props.options.filter((option) => !option.disabled)
  if (!available.length) return
  const index = available.findIndex((option) => option.value === model.value)
  const next =
    available[
      index < 0
        ? delta > 0
          ? 0
          : available.length - 1
        : (index + delta + available.length) % available.length
    ]
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
  <div
    v-else
    ref="root"
    class="relative"
    :class="[open && 'z-20', floating && 'w-full min-w-0']"
    @keydown.esc="open = false"
  >
    <button
      type="button"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-label="label"
      :disabled="disabled"
      class="inline-flex h-8 cursor-pointer items-center gap-2 rounded-full bg-rune-sunken pr-3 pl-4 text-rune-body text-rune-ink transition-colors hover:bg-black disabled:cursor-default disabled:opacity-40"
      :class="floating && 'w-full min-w-0 justify-between'"
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
      <span :class="floating && 'min-w-0 truncate'">{{ selectedLabel() }}</span>
      <AppIcon name="chevron" size="sm" class="shrink-0 text-rune-text-3" />
    </button>
    <ul
      v-if="open && !floating"
      ref="popup"
      role="listbox"
      :aria-label="label"
      class="glass flex flex-col gap-0.5 rounded-rune-tile bg-rune-raised p-1"
      :class="
        floating
          ? 'fixed z-100 overflow-y-auto shadow-xl'
          : 'absolute top-full right-0 mt-1 min-w-full'
      "
      :style="floating ? floatingStyle : undefined"
      @keydown.esc.stop="open = false"
    >
      <li v-for="option in options" :key="String(option.value)" role="presentation">
        <button
          type="button"
          role="option"
          :disabled="option.disabled"
          :aria-selected="option.value === model"
          class="flex w-full cursor-pointer items-center justify-between gap-4 rounded-full px-3 py-2 text-left text-rune-body transition-colors hover:bg-white/8 disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent"
          :title="option.label"
          :class="[
            option.value === model ? 'text-rune-ink' : 'text-rune-text-2',
            !floating && 'whitespace-nowrap',
          ]"
          @click="pick(option.value)"
        >
          <span class="flex min-w-0 items-center gap-2" :class="floating && 'break-all'">
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
    <Teleport v-if="floating" to="body">
      <ul
        v-if="open && floating"
        ref="popup"
        role="listbox"
        :aria-label="label"
        class="glass flex flex-col gap-0.5 rounded-rune-tile bg-rune-raised p-1"
        :class="
          floating
            ? 'fixed z-100 overflow-y-auto shadow-xl'
            : 'absolute top-full right-0 mt-1 min-w-full'
        "
        :style="floating ? floatingStyle : undefined"
        @keydown.esc.stop="open = false"
      >
        <li v-for="option in options" :key="String(option.value)" role="presentation">
          <button
            type="button"
            role="option"
            :disabled="option.disabled"
            :aria-selected="option.value === model"
            class="flex w-full cursor-pointer items-center justify-between gap-4 rounded-full px-3 py-2 text-left text-rune-body transition-colors hover:bg-white/8 disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent"
            :title="option.label"
            :class="[
              option.value === model ? 'text-rune-ink' : 'text-rune-text-2',
              !floating && 'whitespace-nowrap',
            ]"
            @click="pick(option.value)"
          >
            <span class="flex min-w-0 items-center gap-2" :class="floating && 'break-all'">
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
    </Teleport>
  </div>
</template>