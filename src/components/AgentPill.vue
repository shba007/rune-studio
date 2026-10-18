<script setup lang="ts">
import { onKeyStroke } from '@vueuse/core'
import { computed } from 'vue'

import { useAgent } from '@/composables/useAgent'
import { PAGE_GAP, PAGE_WIDTH } from '@/config/layout'

import Composer from './Composer.vue'
import GlyphOrb from './GlyphOrb.vue'
import SettingsPanel from './SettingsPanel.vue'

const agent = useAgent()

const isAmbient = computed(() => agent.mode.value === 'ambient')

/** Backend trouble shows as a dot on the disc; a healthy backend shows nothing. */
const backendDot = computed(() => {
  const status = agent.backend.value
  if (!status) return null
  if (!status.reachable) return 'bg-[#d71921]'
  if (!status.authOk || status.missingModels.length > 0 || status.error) return 'bg-amber-400'
  return null
})

// Esc dismisses the card from anywhere in the window, not only while a control has focus.
onKeyStroke('Escape', () => void agent.dismiss())

/** Idle: the orb opens and closes the card. While the agent runs it is the Stop button. */
function onOrb() {
  if (agent.isRunning.value) void agent.stop()
  else void agent.toggleOpen()
}
</script>

<template>
  <div class="flex h-full w-full items-end justify-end">
    <div
      data-tauri-drag-region
      class="flex size-16 shrink-0 items-center justify-center rounded-full bg-[#141414]"
    >
      <button
        type="button"
        class="relative size-14 cursor-pointer rounded-full bg-[#f1f1ef] outline-none focus-visible:ring-2 focus-visible:ring-white/60"
        :aria-label="agent.isRunning.value ? 'Stop' : isAmbient ? 'Open' : 'Close'"
        @click="onOrb"
      >
        <GlyphOrb :state="agent.state.value" :level="agent.level.value" />
        <span
          v-if="backendDot"
          class="absolute right-0 bottom-0 size-2 rounded-full ring-2 ring-[#f1f1ef]"
          :class="backendDot"
        />
      </button>
    </div>

    <!-- Page column. Every page has the same fixed width, so switching pages never reflows. -->
    <div
      class="h-full overflow-hidden transition-[width] duration-300 ease-out"
      :style="{ width: isAmbient ? '0px' : `${PAGE_WIDTH + PAGE_GAP}px` }"
    >
      <div class="ml-2 flex h-full flex-col justify-end" :style="{ width: `${PAGE_WIDTH}px` }">
        <Transition name="page" mode="out-in">
          <SettingsPanel v-if="agent.mode.value === 'settings'" key="settings" :agent="agent" />
          <Composer v-else key="composer" :agent="agent" />
        </Transition>
      </div>
    </div>
  </div>
</template>