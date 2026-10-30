<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'

import { useReminderSound } from '@/config/alarmSound'

import HubGroup from './HubGroup.vue'
import HubToggle from './HubToggle.vue'

const sound = useReminderSound()
const autostart = ref(false)
const ready = ref(false)
const error = ref('')

onMounted(async () => {
  try {
    autostart.value = await invoke<boolean>('alarms_autostart')
    ready.value = true
  } catch (e) {
    error.value = String(e)
  }
})

async function setAutostart(enabled: boolean) {
  error.value = ''
  try {
    autostart.value = await invoke<boolean>('alarms_set_autostart', { enabled })
  } catch (e) {
    error.value = String(e)
  }
}
</script>

<template>
  <HubGroup title="Alarms">
    <HubToggle
      v-model="sound"
      label="Reminder sound"
      hint="Play a short chime when a scheduled reminder is due."
    />
    <HubToggle
      :model-value="autostart"
      label="Start Rune with Windows"
      hint="Reminders only ring while Rune is running. A toast appears when the orb is hidden."
      :disabled="!ready"
      @update:model-value="setAutostart"
    />
    <p v-if="error" role="alert" class="m-0 text-rune-caption text-error">{{ error }}</p>
  </HubGroup>
</template>