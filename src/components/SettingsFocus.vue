<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'

import { DEFAULT_PRESET, type FocusPreset, type FocusSnapshot } from '@/config/focus'

import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import RuneButton from './RuneButton.vue'

const presets = ref<FocusPreset[]>([DEFAULT_PRESET])
const current = ref<FocusPreset>(DEFAULT_PRESET)
const pending = ref(false)
const error = ref('')

function take(snapshot: FocusSnapshot) {
  presets.value = snapshot.presets
  current.value = snapshot.preset
}

onMounted(async () => {
  try {
    take(await invoke<FocusSnapshot>('focus_snapshot'))
  } catch (e) {
    error.value = String(e)
  }
})

async function choose(id: string) {
  if (pending.value || id === current.value.id) return
  pending.value = true
  error.value = ''
  try {
    take(await invoke<FocusSnapshot>('focus_preset_set', { id }))
  } catch (e) {
    error.value = String(e)
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <HubGroup title="Focus">
    <HubRow
      label="Focus preset"
      hint="How long blocks run, how much time to leave before a commitment, and what the done-when line is called."
    >
      <RuneButton
        v-for="preset in presets"
        :key="preset.id"
        size="sm"
        :variant="preset.id === current.id ? 'primary' : 'secondary'"
        :aria-pressed="preset.id === current.id"
        :disabled="pending"
        @click="choose(preset.id)"
      >
        {{ preset.label }}
      </RuneButton>
    </HubRow>
    <p class="m-0 px-rune-2 font-rune-reading text-rune-caption text-rune-text-3">
      {{ current.summary }} Blocks up to {{ current.capMinutes }} min,
      {{ current.bufferMinutes }} min kept clear before a commitment, a wrap-up warning
      {{ current.wrapUpMinutes }} min ahead, Extend adds {{ current.extendMinutes }} min.
    </p>
    <p v-if="error" role="alert" class="m-0 text-rune-caption text-error">{{ error }}</p>
  </HubGroup>
</template>