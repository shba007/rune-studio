<script setup lang="ts">
import { onClickOutside } from '@vueuse/core'
import { ref, useTemplateRef } from 'vue'

import AppIcon from './AppIcon.vue'
import RuneIconButton from './RuneIconButton.vue'

defineProps<{
  label: string
  items: { id: string; label: string; disabled?: boolean; danger?: boolean }[]
  disabled?: boolean
}>()
const emit = defineEmits<{ select: [id: string] }>()

const open = ref(false)
const root = useTemplateRef<HTMLElement>('root')
onClickOutside(root, () => (open.value = false))

function pick(id: string) {
  open.value = false
  emit('select', id)
}
</script>

<template>
  <div ref="root" class="relative" :class="open && 'z-20'" @keydown.esc="open = false">
    <RuneIconButton
      variant="ghost"
      size="sm"
      :label="label"
      aria-haspopup="menu"
      :aria-expanded="open"
      :disabled="disabled"
      @click="open = !open"
    >
      <AppIcon name="more" />
    </RuneIconButton>
    <ul
      v-if="open"
      role="menu"
      :aria-label="label"
      class="glass absolute top-full right-0 m-0 mt-1 flex min-w-52 list-none flex-col gap-0.5 rounded-rune-tile bg-rune-raised p-1"
    >
      <li v-for="item in items" :key="item.id" role="presentation">
        <button
          type="button"
          role="menuitem"
          :disabled="item.disabled"
          class="flex w-full cursor-pointer items-center rounded-full px-3 py-2 text-left text-rune-body whitespace-nowrap transition-colors hover:bg-white/8 disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent"
          :class="item.danger ? 'text-error' : 'text-rune-text-2 hover:text-rune-ink'"
          @click="pick(item.id)"
        >
          {{ item.label }}
        </button>
      </li>
    </ul>
  </div>
</template>