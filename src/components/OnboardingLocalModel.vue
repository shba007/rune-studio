<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import DownloadProgress from './DownloadProgress.vue'
import RuneButton from './RuneButton.vue'

type Candidate = {
  id: string
  name: string
  license: string
  bytes: number
  availableBytes?: number | null
  revision: string
  manifest: string
}
type Status = {
  phase: string
  model?: string
  filename?: string
  downloaded: number
  total?: number
  error?: string
}
const emit = defineEmits<{ ready: [] }>()
const models = ref<Candidate[]>([])
const status = ref<Status | null>(null)
const error = ref('')
const loading = ref(true)
const starting = ref(false)
const active = computed(() =>
  ['preparing', 'downloading', 'engine'].includes(status.value?.phase ?? ''),
)
let timer: ReturnType<typeof setInterval> | undefined
let disposed = false
let selecting = false
async function refresh() {
  try {
    const next = await invoke<Status>('embedded_model_download_status')
    if (disposed) return
    status.value = next
    if (!active.value && timer) {
      clearInterval(timer)
      timer = undefined
    }
    if (next?.phase === 'complete' && next.model && !selecting) {
      selecting = true
      starting.value = true
      try {
        await invoke('embedded_model_use', { id: next.model })
        if (!disposed) emit('ready')
      } catch (e) {
        if (!disposed) error.value = String(e)
      } finally {
        starting.value = false
      }
    }
  } catch (e) {
    if (!disposed) error.value = String(e)
  }
}
function poll() {
  if (!timer) timer = setInterval(() => void refresh(), 1000)
}
async function download(model: Candidate) {
  error.value = ''
  selecting = false
  try {
    await invoke('embedded_model_download', {
      id: model.id,
      revision: model.revision,
      manifest: model.manifest,
    })
    await refresh()
    if (active.value) poll()
  } catch (e) {
    error.value = String(e)
  }
}
async function cancel() {
  try {
    await invoke('embedded_model_download_cancel')
  } catch (e) {
    error.value = String(e)
  }
}
async function retryStart() {
  selecting = false
  error.value = ''
  await refresh()
}
onMounted(async () => {
  try {
    const available = await invoke<Candidate[]>('embedded_model_candidates')
    models.value = Array.isArray(available) ? available : []
    await refresh()
    if (active.value) poll()
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
})
onBeforeUnmount(() => {
  disposed = true
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="local-setup">
    <p v-if="loading">Loading models…</p>
    <template v-else>
      <p>Models run on your computer. Download sizes exclude the engine and temporary files.</p>
      <p>
        Qwen3.8 27B was tested with Studio; the smaller model still needs computer-use validation.
      </p>
      <p v-if="models[0]?.availableBytes != null">
        {{ (models[0].availableBytes / 1_000_000_000).toFixed(2) }} GB available on the model drive.
      </p>
      <p v-if="!models.length && !error">No compatible models are available in this build.</p>
      <div v-for="model in models" :key="model.id" class="model-row">
        <span
          >{{ model.name }} · {{ (model.bytes / 1_000_000_000).toFixed(2) }} GB ·
          {{ model.license }}</span
        >
        <RuneButton size="sm" :disabled="active || starting" @click="download(model)"
          >Download</RuneButton
        >
      </div>
      <DownloadProgress v-if="active && status" v-bind="status" />
      <RuneButton v-if="active" size="sm" @click="cancel">Cancel</RuneButton>
      <p v-if="starting" role="status">Starting local Infer…</p>
      <RuneButton
        v-if="status?.phase === 'complete' && error && !starting"
        size="sm"
        @click="retryStart"
        >Try starting again</RuneButton
      >
      <p v-if="status?.phase === 'cancelled'" role="status">Download cancelled. You can retry.</p>
      <p v-if="status?.phase === 'failed'" role="alert">
        {{ status.error || 'Download failed. You can retry.' }}
      </p>
    </template>
    <p v-if="error" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.local-setup {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  max-height: 60vh;
  overflow-y: auto;
}
.local-setup p {
  margin: 0;
}
.local-setup [role='alert'] {
  color: var(--color-error);
}
.model-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
</style>