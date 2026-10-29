<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { friendly } from '@/config/errors'
import type { ScreenRecording } from '@/types/protocol'
import CaptureTargetPicker from './CaptureTargetPicker.vue'

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
const button =
  'cursor-pointer rounded-full px-4 py-2 text-rune-caption text-rune-ink disabled:cursor-not-allowed disabled:opacity-40 bg-rune-raised'
</script>
<template>
  <div class="flex flex-col gap-3" aria-label="Screen recording">
    <p class="text-rune-caption text-rune-text-2">
      Silent video · 2 fps · up to 60 seconds. Saved in Rune Studio’s recording folder. Screen
      Insight captures stay separate.
    </p>
    <template v-if="!busy">
      <CaptureTargetPicker
        v-model="target"
        :options="options"
        :refreshing="refreshing"
        @refresh="refresh"
      />
      <p v-if="refreshStatus" role="status" class="text-rune-caption text-rune-text-2">
        {{ refreshStatus }}
      </p>
      <label class="flex items-center justify-between text-rune-caption text-rune-text-2"
        >Duration (seconds)
        <input
          v-model.number="seconds"
          type="number"
          min="1"
          max="60"
          class="w-20 rounded-rune-tile bg-rune-surface px-3 py-2 text-rune-ink"
        />
      </label>
      <RuneButton size="sm" :disabled="!target || pending" @click="start">
        Start recording
      </RuneButton>
    </template>
    <p v-if="busy" role="status" class="text-rune-caption text-rune-text-2">
      {{
        status.phase === 'saving'
          ? 'Saving video…'
          : status.phase === 'starting'
            ? 'Starting recording…'
            : `Recording · ${status.seconds}s · Escape stops and saves`
      }}
    </p>
    <div v-if="busy" class="flex gap-2">
      <RuneButton size="sm" :disabled="status.phase === 'saving'" @click="stop(false)">
        Stop and save
      </RuneButton>
      <RuneButton size="sm" @click="stop(true)">Cancel and discard</RuneButton>
    </div>
    <p v-if="status.path" class="text-rune-caption break-all text-rune-text-2 select-text">
      Saved: {{ status.path }}
    </p>
    <p v-if="status.phase === 'cancelled'" class="text-rune-caption text-rune-text-2">
      Recording discarded. No video saved.
    </p>
    <p
      v-if="notice || (status.error && status.phase === 'failed')"
      role="alert"
      class="text-rune-caption text-error"
    >
      {{ notice || status.error }}
    </p>
  </div>
</template>