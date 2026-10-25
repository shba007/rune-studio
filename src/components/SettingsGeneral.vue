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

defineProps<{ agent: Agent; draft: Settings }>()

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
    <HubGroup title="Screen capture">
      <HubRow
        label="Model image resolution"
        hint="Limits the longest edge sent to models for screen analysis and desktop tasks. Larger images use more memory and may take longer or exceed the model's limit. Saved screenshots keep their native resolution."
      >
        <select
          v-model="draft.modelImageResolution"
          aria-label="Model image resolution"
          class="rounded-lg bg-zinc-900 px-3 py-2 text-body text-zinc-100"
        >
          <option value="720">720 px · Faster</option>
          <option value="1280">1280 px · Balanced</option>
          <option value="1920">1920 px · Detailed</option>
          <option value="2560">2560 px · More detailed</option>
          <option value="native">Native resolution</option>
        </select>
      </HubRow>
    </HubGroup>
    <HubGroup title="Reflection">
      <p class="m-0 text-label text-zinc-500">Self reflection is unavailable in this release.</p>
    </HubGroup>

    <HubGroup title="Recording">
      <SettingsReview :draft="draft" />
      <HubToggle
        v-model="draft.demoMode"
        label="Show Rune in screen recordings"
        hint="OBS, Game Bar and the Snipping Tool can capture Rune's windows. Rune hides itself from its own model either way."
      />
    </HubGroup>

    <HubGroup title="Screen recording">
      <HubRow
        label="Recording resolution"
        hint="Applies to new screen and session recordings. Limits the longest edge without enlarging smaller captures. Native retains captured dimensions; video stays silent at 2 fps."
      >
        <select
          v-model="draft.recordingResolution"
          aria-label="Recording resolution"
          class="rounded-lg bg-zinc-900 px-3 py-2 text-body text-zinc-100"
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

    <HubGroup title="Safety">
      <HubToggle v-model="draft.askEveryAction" label="Ask before every action" />
    </HubGroup>

    <HubGroup title="Developer">
      <HubToggle
        v-model="draft.developerMode"
        label="Developer mode"
        hint="Show screen capture, input tests, animation previews and raw session records."
      />
    </HubGroup>

    <details
      v-if="draft.developerMode"
      ref="developer"
      class="rounded-2xl bg-white/5 px-5 py-3"
      @toggle="revealDeveloper"
    >
      <summary class="cursor-pointer text-caption text-zinc-500 uppercase">
        Capture, input and animation
      </summary>
      <div class="flex flex-wrap gap-2 pt-3">
        <button
          v-for="s in DEBUG_STATES"
          :key="s"
          type="button"
          class="h-8 cursor-pointer rounded-full border-0 bg-white/8 px-3 text-label text-zinc-300 transition hover:bg-white/15"
          @click="agent.debugSetState(s)"
        >
          {{ s }}
        </button>
      </div>
      <DevInputLab :agent="agent" class="mt-3" />
    </details>
  </div>
</template>