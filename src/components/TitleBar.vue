<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { onBeforeUnmount, onMounted, ref } from 'vue'

import AppIcon from './AppIcon.vue'

withDefaults(defineProps<{ title: string; controls?: 'all' | 'close' }>(), { controls: 'all' })

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
  >
    <div data-tauri-drag-region class="pointer-events-none flex items-center gap-2 pl-4">
      <img :src="LOGO" alt="" class="size-5" />
      <span class="text-rune-heading text-rune-text-2 uppercase">{{ title }}</span>
    </div>
    <div class="flex h-full">
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