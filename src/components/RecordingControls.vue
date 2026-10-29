<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { friendly } from '@/config/errors'
import type { ScreenRecording } from '@/types/protocol'
import CaptureTargetPicker from './CaptureTargetPicker.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'

const options = ref<{ target: { kind: string; id: number }; label: string }[]>([])
const target = ref('')
const seconds = ref(10)
const status = ref<ScreenRecording>({
  id: 0,
  phase: 'idle',
  seconds: 0,
  monitor: null,
  path: null,
  error: null,
})
const notice = ref('')
const pending = ref(false)
const refreshing = ref(false)
const refreshStatus = ref('')
const busy = computed(() => ['starting', 'recording', 'saving'].includes(status.value.phase))
let off: UnlistenFn | undefined
async function refresh() {
  if (refreshing.value) return
  refreshing.value = true
  notice.value = ''
  refreshStatus.value = ''
  try {
    const available = await invoke('screen_insight_targets')
    options.value = Array.isArray(available) ? available : []
    refreshStatus.value = `${options.value.length} capture targets available`
    if (!options.value.some((o) => JSON.stringify(o.target) === target.value)) target.value = ''
  } catch (e) {
    notice.value = friendly(String(e))
  } finally {
    refreshing.value = false
  }
}
async function start() {
  const selected = options.value.find((o) => JSON.stringify(o.target) === target.value)
  if (!selected || busy.value || pending.value) return
  notice.value = ''
  pending.value = true
  try {
    status.value = await invoke('start_screen_recording', {
      target: selected.target,
      seconds: seconds.value,
    })
  } catch (e) {
    notice.value = friendly(String(e))
  } finally {
    pending.value = false
  }
}
async function stop(discard: boolean) {
  try {
    await invoke('stop_screen_recording', { discard })
  } catch (e) {
    notice.value = friendly(String(e))
  }
}
onMounted(async () => {
  off = await listen<ScreenRecording>('screen_recording', (event) => {
    status.value = event.payload
  })
  try {
    const latest = await invoke<ScreenRecording>('screen_recording_state')
    if (latest && typeof latest.phase === 'string') status.value = latest
  } catch {}
  await refresh()
})
onBeforeUnmount(() => off?.())
</script>
<template>
  <HubGroup
    title="Screen recording"
    hint="Silent video · 2 fps · up to 60 seconds. Saved in Rune Studio’s recording folder. Screen Insight captures stay separate."
    aria-label="Screen recording"
  >
    <template v-if="!busy">
      <HubRow
        label="Capture target"
        :hint="refreshing ? 'Refreshing…' : refreshStatus || 'Choose a screen or window.'"
      >
        <CaptureTargetPicker
          v-model="target"
          :options="options"
          :refreshing="refreshing"
          @refresh="refresh"
        />
      </HubRow>
      <HubRow label="Duration" hint="Seconds, from 1 to 60.">
        <input
          v-model.number="seconds"
          type="number"
          min="1"
          max="60"
          aria-label="Duration (seconds)"
          class="h-10 w-24 rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink"
        />
      </HubRow>
      <HubRow label="Record" hint="Starts right away. Escape stops and saves.">
        <RuneButton size="sm" :disabled="!target || pending" @click="start">
          Start recording
        </RuneButton>
      </HubRow>
    </template>
    <HubRow v-else label="Recording">
      <RuneButton size="sm" :disabled="status.phase === 'saving'" @click="stop(false)">
        Stop and save
      </RuneButton>
      <RuneButton variant="ghost" size="sm" @click="stop(true)">Cancel and discard</RuneButton>
    </HubRow>
    <template #after>
      <p v-if="busy" role="status" class="m-0 px-rune-2 text-rune-caption text-rune-text-2">
        {{
          status.phase === 'saving'
            ? 'Saving video…'
            : status.phase === 'starting'
              ? 'Starting recording…'
              : `Recording · ${status.seconds}s · Escape stops and saves`
        }}
      </p>
      <p
        v-if="status.path"
        class="m-0 px-rune-2 text-rune-caption break-all text-rune-text-2 select-text"
      >
        Saved: {{ status.path }}
      </p>
      <p
        v-if="status.phase === 'cancelled'"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-2"
      >
        Recording discarded. No video saved.
      </p>
      <p
        v-if="notice || (status.error && status.phase === 'failed')"
        role="alert"
        class="m-0 px-rune-2 text-rune-caption text-error"
      >
        {{ notice || status.error }}
      </p>
    </template>
  </HubGroup>
</template>