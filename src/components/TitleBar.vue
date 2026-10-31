<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { onClickOutside } from '@vueuse/core'
import { onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'

import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneIconButton from './RuneIconButton.vue'

withDefaults(
  defineProps<{
    title: string
    controls?: 'all' | 'close'
    sidebar?: boolean
    bare?: boolean
    collapsed?: boolean
    canBack?: boolean
    canForward?: boolean
    menu?: { id: string; label: string }[]
  }>(),
  {
    controls: 'all',
    sidebar: false,
    bare: false,
    collapsed: false,
    canBack: false,
    canForward: false,
    menu: () => [],
  },
)
const emit = defineEmits<{
  toggle: []
  back: []
  forward: []
  select: [id: string]
}>()

const menuOpen = ref(false)
const menuRoot = useTemplateRef<HTMLElement>('menuRoot')
onClickOutside(menuRoot, () => (menuOpen.value = false))

const LOGO = '/logo.svg'

const maximized = ref(false)
let unlisten: (() => void) | null = null

function win() {
  try {
    return getCurrentWindow()
  } catch {
    return null
  }
}

async function sync() {
  try {
    maximized.value = (await win()?.isMaximized()) ?? false
  } catch {
    maximized.value = false
  }
}

onMounted(async () => {
  await sync()
  try {
    unlisten = (await win()?.onResized(sync)) ?? null
  } catch {
    unlisten = null
  }
})
onBeforeUnmount(() => unlisten?.())

const button =
  'grid h-full w-12 cursor-pointer place-items-center bg-transparent text-rune-text-2 transition hover:bg-white/10 hover:text-rune-ink'
</script>

<template>
  <header
    data-tauri-drag-region
    class="flex h-10 shrink-0 select-none items-center justify-between"
    :class="sidebar && 'bg-black'"
  >
    <div
      v-if="sidebar"
      class="flex h-full shrink-0 items-center gap-1 px-3"
      :class="collapsed ? 'w-auto' : 'w-64'"
    >
      <div ref="menuRoot" class="relative">
        <RuneIconButton variant="ghost" size="sm" label="Menu" @click="menuOpen = !menuOpen">
          <AppIcon name="menu" />
        </RuneIconButton>
        <ul
          v-if="menuOpen"
          role="menu"
          class="glass absolute top-full left-0 z-30 mt-1 flex min-w-44 flex-col gap-0.5 rounded-rune-tile bg-rune-raised p-1"
        >
          <li v-for="item in menu" :key="item.id" role="presentation">
            <RuneButton
              role="menuitem"
              variant="ghost"
              size="sm"
              class="w-full justify-start"
              @click="((menuOpen = false), emit('select', item.id))"
            >
              {{ item.label }}
            </RuneButton>
          </li>
        </ul>
      </div>
      <RuneIconButton
        variant="ghost"
        size="sm"
        :label="collapsed ? 'Show sidebar' : 'Hide sidebar'"
        @click="emit('toggle')"
      >
        <AppIcon name="drawer" />
      </RuneIconButton>
      <RuneIconButton
        variant="ghost"
        size="sm"
        label="Back"
        :disabled="!canBack"
        @click="emit('back')"
      >
        <AppIcon name="back" />
      </RuneIconButton>
      <RuneIconButton
        variant="ghost"
        size="sm"
        label="Forward"
        :disabled="!canForward"
        @click="emit('forward')"
      >
        <AppIcon name="forward" />
      </RuneIconButton>
    </div>
    <div
      v-else-if="!bare"
      data-tauri-drag-region
      class="pointer-events-none flex items-center gap-2 pl-4"
    >
      <img :src="LOGO" alt="" class="size-5" />
      <span class="text-rune-heading text-rune-text-2 uppercase">{{ title }}</span>
    </div>
    <div class="ml-auto flex h-full">
      <button
        v-if="controls === 'all'"
        type="button"
        :class="button"
        aria-label="Minimise"
        @click="win()?.minimize()"
      >
        <AppIcon name="minimize" />
      </button>
      <button
        v-if="controls === 'all'"
        type="button"
        :class="button"
        :aria-label="maximized ? 'Restore' : 'Maximise'"
        @click="win()?.toggleMaximize()"
      >
        <AppIcon :name="maximized ? 'restore' : 'maximize'" />
      </button>
      <button
        type="button"
        :class="[button, 'hover:bg-attention! hover:text-white!']"
        aria-label="Close"
        @click="win()?.close()"
      >
        <AppIcon name="close" />
      </button>
    </div>
  </header>
</template>