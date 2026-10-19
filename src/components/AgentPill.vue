<script setup lang="ts">
import { onKeyStroke, useElementSize } from '@vueuse/core'
import { computed, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import { useUpdater } from '@/composables/useUpdater'
import { PAGE_GAP, PAGE_WIDTH } from '@/config/layout'

import Composer from './Composer.vue'
import ConfirmCard from './ConfirmCard.vue'
import GlyphOrb from './GlyphOrb.vue'
import HealthDots from './HealthDots.vue'
import ProgressRing from './ProgressRing.vue'
import SettingsPanel from './SettingsPanel.vue'
import TaskPanel from './TaskPanel.vue'

const agent = useAgent()
const updater = useUpdater()

/** An update is being installed: the rim shows that instead of the task's goals. */
const updating = updater.updating
/** Percent of the download for the rim, or null while the size is unknown. */
const downloadPercent = computed(() =>
  updater.phase.value === 'downloading' && updater.progress.value !== null
    ? Math.round(updater.progress.value * 100)
    : null,
)
/** The prompt waits for a free moment: never over a running task or a question. */
const promptVisible = computed(
  () =>
    updater.prompt.value &&
    updater.phase.value === 'available' &&
    !agent.isRunning.value &&
    !agent.question.value,
)
// A found update opens the card by itself so the prompt is seen.
watch(promptVisible, (visible) => {
  if (visible && agent.mode.value === 'ambient') void agent.setMode('open')
})

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
  if (updating.value) return
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
      <svg v-if="updating" viewBox="0 0 64 64" class="update-ring" aria-hidden="true">
        <circle cx="32" cy="32" r="30" fill="none" class="update-track" />
        <circle
          v-if="downloadPercent !== null"
          cx="32"
          cy="32"
          r="30"
          fill="none"
          pathLength="100"
          class="update-fill"
          :stroke-dasharray="`${downloadPercent} 100`"
        />
        <circle
          v-else-if="updater.phase.value !== 'restarting'"
          cx="32"
          cy="32"
          r="30"
          fill="none"
          pathLength="100"
          class="update-fill update-sweep"
          stroke-dasharray="28 100"
        />
        <circle
          v-else
          cx="32"
          cy="32"
          r="30"
          fill="none"
          pathLength="100"
          class="update-fill"
          stroke-dasharray="100 100"
        />
      </svg>
      <ProgressRing
        v-else
        :goals="agent.plan.value?.goals.length ?? 0"
        :current="agent.plan.value?.current ?? 0"
        :finished="agent.plan.value?.finished ?? false"
        :running="agent.isRunning.value"
        :failed="agent.state.value === 'failed'"
      />
      <button
        type="button"
        class="relative size-14 cursor-pointer rounded-full bg-[#f1f1ef] outline-none focus-visible:ring-2 focus-visible:ring-white/60"
        :aria-label="
          updating ? 'Updating' : agent.isRunning.value ? 'Stop' : isAmbient ? 'Open' : 'Close'
        "
        :title="updating ? 'Updating Rune Studio' : agent.health.value.title"
        @click="onOrb"
      >
        <GlyphOrb :state="updating ? 'planning' : agent.state.value" :level="agent.level.value" />
        <HealthDots :tone="agent.health.value.tone" />
      </button>
    </div>

    <!-- Page column. Every page has the same fixed width, so switching pages never reflows. -->
    <div
      class="h-full overflow-hidden transition-[width] duration-300 ease-out"
      :style="{ width: isAmbient ? '0px' : `${PAGE_WIDTH + PAGE_GAP}px` }"
    >
      <div class="ml-2 flex h-full flex-col justify-end" :style="{ width: `${PAGE_WIDTH}px` }">
        <!-- Settings fills the window and scrolls inside; the other pages are as tall as their content. -->
        <div
          ref="content"
          class="flex flex-col"
          :class="agent.mode.value === 'settings' ? 'h-full min-h-0' : ''"
        >
          <Transition name="page" mode="out-in">
            <SettingsPanel v-if="agent.mode.value === 'settings'" key="settings" :agent="agent" />
            <div v-else key="main" class="flex flex-col gap-2">
              <Transition name="page">
                <TaskPanel v-if="agent.mode.value === 'task'" :agent="agent" />
              </Transition>
              <section
                v-if="promptVisible && updater.latest.value"
                class="flex w-full flex-col gap-2 rounded-[28px] border border-white/15 bg-[#141414] p-4 text-zinc-100"
                role="alertdialog"
                aria-label="Update available"
              >
                <header class="flex h-6 items-center gap-2">
                  <span class="size-2 shrink-0 rounded-full bg-[#f1f1ef]" />
                  <span class="text-caption text-zinc-300 uppercase">Update available</span>
                </header>
                <p class="text-body">
                  Version {{ updater.latest.value.version }}
                  <span class="text-zinc-500">(you have {{ updater.version.value }})</span>
                </p>
                <div class="mt-1 flex gap-2">
                  <button
                    type="button"
                    class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 text-label text-zinc-200 transition hover:bg-white/10"
                    @click="updater.dismissPrompt()"
                  >
                    Later
                  </button>
                  <button
                    type="button"
                    class="h-8 flex-1 cursor-pointer rounded-full bg-[#f1f1ef] text-label text-[#111] transition hover:bg-white"
                    @click="updater.install(agent.isRunning.value)"
                  >
                    Update
                  </button>
                </div>
              </section>
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

<style scoped>
/* The update ring is deliberately unlike the goal ring: one solid, thin line. */
.update-ring {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  transform: rotate(-90deg);
}

.update-track {
  stroke: rgb(241 241 239 / 0.18);
  stroke-width: 2;
}

.update-fill {
  stroke: #f1f1ef;
  stroke-width: 2.5;
  stroke-linecap: round;
  transition: stroke-dasharray 250ms ease-out;
}

.update-sweep {
  transform-origin: 32px 32px;
  animation: update-spin 1.1s linear infinite;
}

@keyframes update-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .update-sweep {
    animation: none;
  }
}
</style>