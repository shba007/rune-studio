<script setup lang="ts">
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
  'cursor-pointer rounded-full border border-white/20 px-4 py-2 text-label text-zinc-200 disabled:cursor-not-allowed disabled:opacity-40'
</script>
<template>
  <div class="flex flex-col gap-3" aria-label="Screen recording">
    <p class="text-label text-zinc-400">
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
      <p v-if="refreshStatus" role="status" class="text-label text-zinc-400">{{ refreshStatus }}</p>
      <label class="flex items-center justify-between text-label text-zinc-300"
        >Duration (seconds)
        <input
          v-model.number="seconds"
          type="number"
          min="1"
          max="60"
          class="w-20 rounded-lg bg-white/8 px-3 py-2 text-zinc-100"
        />
      </label>
      <button type="button" :class="button" :disabled="!target || pending" @click="start">
        Start recording
      </button>
    </template>
    <p v-if="busy" role="status" class="text-label text-zinc-300">
      {{
        status.phase === 'saving'
          ? 'Saving video…'
          : status.phase === 'starting'
            ? 'Starting recording…'
            : `Recording · ${status.seconds}s · Escape stops and saves`
      }}
    </p>
    <div v-if="busy" class="flex gap-2">
      <button
        type="button"
        :class="button"
        :disabled="status.phase === 'saving'"
        @click="stop(false)"
      >
        Stop and save
      </button>
      <button type="button" :class="button" @click="stop(true)">Cancel and discard</button>
    </div>
    <p v-if="status.path" class="text-label break-all text-zinc-300 select-text">
      Saved: {{ status.path }}
    </p>
    <p v-if="status.phase === 'cancelled'" class="text-label text-zinc-400">
      Recording discarded. No video saved.
    </p>
    <p
      v-if="notice || (status.error && status.phase === 'failed')"
      role="alert"
      class="text-label text-[#ff6b72]"
    >
      {{ notice || status.error }}
    </p>
  </div>
</template>