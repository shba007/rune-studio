<script setup lang="ts">
import { computed } from 'vue'

import { useAgent } from '@/composables/useAgent'

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

/** Idle: the orb opens and closes the card. While the agent runs it is the Stop button. */
function onOrb() {
  if (agent.isRunning.value) void agent.stop()
  else void agent.toggleOpen()
}
</script>

<template>
  <div class="flex h-full w-full items-end justify-end" @keydown.esc="agent.dismiss()">
    <div
      data-tauri-drag-region
      class="flex size-19 shrink-0 items-center justify-center rounded-full bg-[#141414]"
    >
      <button
        type="button"
        class="relative size-16 cursor-pointer rounded-full bg-[#f1f1ef] outline-none focus-visible:ring-2 focus-visible:ring-white/60"
        :aria-label="agent.isRunning.value ? 'Stop' : isAmbient ? 'Open' : 'Close'"
        @click="onOrb"
      >
        <GlyphOrb :state="agent.state.value" :level="agent.level.value" />
        <span
          v-if="backendDot"
          class="absolute right-0.5 bottom-0.5 size-2.5 rounded-full ring-2 ring-[#f1f1ef]"
          :class="backendDot"
        />
      </button>
    </div>

    <!-- Card column. Fixed inner width so content never reflows while the outer width animates. -->
    <div
      class="h-full overflow-hidden transition-[width] duration-300 ease-out"
      :style="{ width: isAmbient ? '0px' : '496px' }"
    >
      <div class="ml-2 flex h-full w-[488px] flex-col justify-end">
        <SettingsPanel v-if="agent.mode.value === 'settings'" :agent="agent" />
        <Composer v-else :agent="agent" />
      </div>
    </div>
  </div>
</template>