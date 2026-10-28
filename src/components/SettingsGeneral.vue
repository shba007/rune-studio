<script setup lang="ts">
import { nextTick, useTemplateRef } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import DevInputLab from './DevInputLab.vue'
import HubGroup from './HubGroup.vue'
import HubToggle from './HubToggle.vue'
import HubRow from './HubRow.vue'
import SettingsReview from './SettingsReview.vue'
import RecordingControls from './RecordingControls.vue'
import SettingsGoalContext from './SettingsGoalContext.vue'

withDefaults(
  defineProps<{
    agent: Agent
    draft: Settings
    view?: 'all' | 'general' | 'models' | 'capture' | 'developer'
  }>(),
  { view: 'all' },
)

const DEBUG_STATES: AgentState[] = [
  'idle',
  'listening',
  'planning',
  'acting',
  'awaiting_confirm',
  'done',
  'failed',
]

const developer = useTemplateRef<HTMLDetailsElement>('developer')
async function revealDeveloper() {
  if (!developer.value?.open) return
  await nextTick()
  developer.value.scrollIntoView({ block: 'end', behavior: 'smooth' })
}
</script>

<template>
  <div class="flex flex-col gap-8">
    <SettingsGoalContext v-if="view === 'all'" :draft="draft" />
    <HubGroup v-if="view === 'all' || view === 'models'" title="Model input">
      <HubRow
        label="Model input resolution"
        hint="Limits images sent to models for faster inference. Independent of saved screenshot and recording quality."
      >
        <select
          v-model="draft.modelImageResolution"
          aria-label="Model input resolution"
          class="rounded-lg bg-zinc-900 px-rune-2 py-2 text-rune-body text-zinc-100"
        >
          <option value="720">720 px · Faster</option>
          <option value="1280">1280 px · Balanced</option>
          <option value="1920">1920 px · Detailed</option>
          <option value="2560">2560 px · More detailed</option>
          <option value="native">Native resolution</option>
        </select>
      </HubRow>
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'capture'" title="Recording">
      <SettingsReview :draft="draft" />
      <HubToggle
        v-model="draft.demoMode"
        label="Show Rune in screen recordings"
        hint="OBS, Game Bar and the Snipping Tool can capture Rune's windows. Rune hides itself from its own model either way."
      />
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'capture'" title="Capture quality">
      <HubRow
        label="Screenshot and recording resolution"
        hint="Quality of new saved screenshots and recordings. Limits the longest edge without enlarging captures. Native preserves captured dimensions. Model input resolution is independent; video remains silent at 2 fps."
      >
        <select
          v-model="draft.recordingResolution"
          aria-label="Screenshot and recording resolution"
          class="rounded-lg bg-zinc-900 px-rune-2 py-2 text-rune-body text-zinc-100"
        >
          <option value="720">720 px</option>
          <option value="1280">1280 px</option>
          <option value="1920">1920 px</option>
          <option value="2560">2560 px</option>
          <option value="native">Native resolution</option>
        </select>
      </HubRow>
      <RecordingControls />
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'general'" title="Safety &amp; behavior">
      <HubToggle
        v-model="draft.askEveryAction"
        label="Ask before every action"
        hint="Request approval for each action, in addition to existing safety checks."
      /><HubToggle
        v-if="view === 'general'"
        v-model="draft.showThinking"
        label="Show thinking"
        hint="Show model thinking in the orb bubble when available."
      />
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'developer'" title="Developer tools">
      <HubToggle
        v-model="draft.developerMode"
        label="Developer mode"
        hint="Show screen capture, input tests, animation previews and raw task records."
      />
    </HubGroup>

    <details
      v-if="(view === 'all' || view === 'developer') && draft.developerMode"
      ref="developer"
      class="rounded-2xl bg-white/5 px-rune-2 py-rune-2"
      @toggle="revealDeveloper"
    >
      <summary class="cursor-pointer text-rune-caption text-zinc-500 uppercase">
        Capture, input and animation
      </summary>
      <div class="flex flex-wrap gap-2 pt-rune-2">
        <button
          v-for="s in DEBUG_STATES"
          :key="s"
          type="button"
          class="h-8 cursor-pointer rounded-full border-0 bg-white/8 px-rune-2 text-rune-body text-zinc-300 transition hover:bg-white/15"
          @click="agent.debugSetState(s)"
        >
          {{ s }}
        </button>
      </div>
      <DevInputLab :agent="agent" class="mt-3" />
    </details>
  </div>
</template>