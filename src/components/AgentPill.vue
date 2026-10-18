<script setup lang="ts">
import { onKeyStroke, useElementSize } from '@vueuse/core'
import { computed, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import { PAGE_GAP, PAGE_WIDTH } from '@/config/layout'

import Composer from './Composer.vue'
import ConfirmCard from './ConfirmCard.vue'
import GlyphOrb from './GlyphOrb.vue'
import HealthDots from './HealthDots.vue'
import ProgressRing from './ProgressRing.vue'
import SettingsPanel from './SettingsPanel.vue'
import TaskPanel from './TaskPanel.vue'

const agent = useAgent()

const isAmbient = computed(() => agent.mode.value === 'ambient')

// Dev only: lets a plain browser preview set the plan, thinking and steps by hand.
if (import.meta.env.DEV) Object.assign(window, { __demoAgent: agent })

// The window is as tall as the open card's content, so it grows when the text does.
const content = useTemplateRef<HTMLElement>('content')
const { height: contentHeight } = useElementSize(content)
watch(contentHeight, (height) => {
  // Between two pages the column is briefly empty; ignore that.
  if (height >= 48) void agent.fitHeight(Math.ceil(height / 8) * 8)
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
      class="relative flex size-16 shrink-0 items-center justify-center rounded-full bg-[#141414]"
    >
      <!-- The rim of the orb is the task's progress: one dotted arc per goal. -->
      <ProgressRing
        :goals="agent.plan.value?.goals.length ?? 0"
        :current="agent.plan.value?.current ?? 0"
        :finished="agent.plan.value?.finished ?? false"
        :running="agent.isRunning.value"
        :failed="agent.state.value === 'failed'"
      />
      <button
        type="button"
        class="relative size-14 cursor-pointer rounded-full bg-[#f1f1ef] outline-none focus-visible:ring-2 focus-visible:ring-white/60"
        :aria-label="agent.isRunning.value ? 'Stop' : isAmbient ? 'Open' : 'Close'"
        :title="agent.health.value.title"
        @click="onOrb"
      >
        <GlyphOrb :state="agent.state.value" :level="agent.level.value" />
        <HealthDots :tone="agent.health.value.tone" />
      </button>
    </div>

    <!-- Page column. Every page has the same fixed width, so switching pages never reflows. -->
    <div
      class="h-full overflow-hidden transition-[width] duration-300 ease-out"
      :style="{ width: isAmbient ? '0px' : `${PAGE_WIDTH + PAGE_GAP}px` }"
    >
      <div class="ml-2 flex h-full flex-col justify-end" :style="{ width: `${PAGE_WIDTH}px` }">
        <div ref="content" class="flex flex-col">
          <Transition name="page" mode="out-in">
            <SettingsPanel v-if="agent.mode.value === 'settings'" key="settings" :agent="agent" />
            <div v-else key="main" class="flex flex-col gap-2">
              <Transition name="page">
                <TaskPanel v-if="agent.mode.value === 'task'" :agent="agent" />
              </Transition>
              <ConfirmCard
                v-if="agent.question.value"
                :key="agent.question.value.id"
                :agent="agent"
              />
              <Composer :agent="agent" />
            </div>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>