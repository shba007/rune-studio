<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'
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
  sourceUrl?: string
  recommended?: boolean
  recommendationBasis?: string | null
}
type Found = { id: string; name: string; bytes: number; path: string }
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
const selectedId = ref('')
const selected = computed(() => models.value.find((model) => model.id === selectedId.value))
const status = ref<Status | null>(null)
const error = ref('')
const loading = ref(true)
const starting = ref(false)
const active = computed(() =>
  ['preparing', 'downloading', 'engine'].includes(status.value?.phase ?? ''),
)
const panel = useTemplateRef<HTMLElement>('panel')
watch(error, async (value) => {
  if (!value) return
  await nextTick()
  panel.value?.scrollTo({ top: panel.value.scrollHeight })
})
const folder = ref('')
const found = ref<Found[]>([])
const scanned = ref(false)
const scanning = ref(false)
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
async function browse() {
  try {
    const picked = await invoke<string | null>('pick_folder')
    if (picked) {
      folder.value = picked
      await scan()
    }
  } catch (e) {
    error.value = String(e)
  }
}
async function scan() {
  const dir = folder.value.trim()
  if (!dir) return
  error.value = ''
  scanning.value = true
  scanned.value = false
  try {
    found.value = await invoke<Found[]>('embedded_model_scan', { dir })
    scanned.value = true
  } catch (e) {
    found.value = []
    error.value = String(e)
  } finally {
    scanning.value = false
  }
}
async function useFound(model: Found) {
  error.value = ''
  selecting = false
  try {
    await invoke('embedded_model_import', { id: model.id, dir: folder.value.trim() })
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
    models.value = Array.isArray(available)
      ? available.filter((model) => model.id.startsWith('holo'))
      : []
    selectedId.value = models.value.find((model) => model.recommended)?.id ?? ''
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
  <div ref="panel" class="local-setup">
    <p v-if="loading">Loading models…</p>
    <template v-else>
      <div class="model-heading">
        <span>Local models</span>
        <span v-if="models[0]?.availableBytes != null"
          >{{ (models[0].availableBytes / 1_000_000_000).toFixed(2) }} GB available</span
        >
      </div>
      <p v-if="!models.length && !error">No compatible models are available in this build.</p>
      <div v-if="models.length" class="model-list" role="radiogroup" aria-label="Local model">
        <label
          v-for="model in models"
          :key="model.id"
          class="model-row"
          :class="{ selected: selectedId === model.id }"
        >
          <input
            v-model="selectedId"
            type="radio"
            name="local-model"
            :value="model.id"
            :disabled="active || starting"
          />
          <span class="selection-dot" aria-hidden="true" />
          <span class="model-info">
            <span class="model-title"
              >{{ model.name }}
              <strong v-if="model.recommended" class="recommended">Recommended</strong></span
            >
            <span class="model-meta">{{ model.license }}</span>
          </span>
          <span class="model-size">{{ (model.bytes / 1_000_000_000).toFixed(2) }} GB</span>
        </label>
      </div>
      <div class="model-details">
        <p v-if="!selectedId && models.length">
          No reliable fit was detected. Select a model to continue.
        </p>
        <p v-if="selected?.sourceUrl" class="source">Source: {{ selected.sourceUrl }}</p>
        <p v-if="models[0]?.recommendationBasis">
          {{ models[0].recommendationBasis }}. Actual performance may vary.
        </p>
        <p>
          Models run on your computer. These Holo models have not yet been validated with Studio.
          Sizes exclude engine and temporary files.
        </p>
      </div>
      <RuneButton
        v-if="models.length"
        size="sm"
        variant="primary"
        class="download-button"
        :disabled="!selected || active || starting"
        @click="selected && download(selected)"
        >Download selected model</RuneButton
      >
      <div class="existing">
        <div class="model-heading"><span>Already have a model?</span></div>
        <p class="hint">Pick the folder that holds your models and Rune will find a match.</p>
        <div class="folder-row">
          <input
            v-model="folder"
            class="folder-input"
            type="text"
            placeholder="Models folder"
            aria-label="Models folder"
            :disabled="active || starting"
            @keydown.enter="scan"
          />
          <RuneButton size="sm" :disabled="active || starting" @click="browse">Browse</RuneButton>
          <RuneButton
            size="sm"
            :disabled="!folder.trim() || scanning || active || starting"
            @click="scan"
            >{{ scanning ? 'Scanning…' : 'Scan' }}</RuneButton
          >
        </div>
        <p v-if="scanned && !found.length" role="status">
          No supported model was found in this folder.
        </p>
        <div v-for="model in found" :key="model.id" class="found-row">
          <span class="model-info">
            <span class="model-title">{{ model.name }}</span>
            <span class="model-meta source">{{ model.path }}</span>
          </span>
          <RuneButton
            size="sm"
            variant="primary"
            :disabled="active || starting"
            @click="useFound(model)"
            >Use</RuneButton
          >
        </div>
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
  max-height: 100%;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 4px 8px 4px 4px;
}
.local-setup p {
  margin: 0;
}
.local-setup [role='alert'] {
  color: var(--color-error);
}
.model-heading {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  color: var(--color-rune-text-3);
  font: 600 11px/1.4 var(--font-sans);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.model-list {
  overflow: hidden;
  border: 1px solid var(--color-rune-raised);
  border-radius: var(--radius-rune-tile);
  background: var(--color-rune-surface);
}
.model-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  cursor: pointer;
  transition: background 150ms ease;
}
.model-row + .model-row {
  border-top: 1px solid var(--color-rune-raised);
}
.model-row:hover,
.model-row.selected {
  background: var(--color-rune-raised);
}
.model-row input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
}
.selection-dot {
  width: 16px;
  height: 16px;
  flex: 0 0 16px;
  border: 1px solid var(--color-rune-text-3);
  border-radius: 50%;
  display: grid;
  place-items: center;
}
.selected .selection-dot {
  border-color: var(--accent);
}
.selected .selection-dot::after {
  content: '';
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
}
.model-row:has(input:focus-visible) {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.model-row:has(input:disabled) {
  cursor: default;
}
.model-row:has(input:disabled):not(.selected) {
  opacity: 0.6;
}
.model-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font: 400 12px/1.4 var(--font-rune-reading);
  color: var(--color-rune-text-3);
}
.model-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 2px;
  flex: 1;
}
.model-title {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 6px;
  font: 600 14px/1.4 var(--font-sans);
  font-weight: 600;
}
.model-size,
.model-meta {
  color: var(--color-rune-text-2);
  font: 400 12px/1.4 var(--font-rune-reading);
}
.model-size {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}
.recommended {
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  border-radius: 999px;
  padding: 2px 7px;
  font: 600 10px/1.4 var(--font-rune-reading);
}
.source {
  overflow-wrap: anywhere;
  user-select: text;
}
.existing {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.hint {
  color: var(--color-rune-text-3);
  font: 400 12px/1.4 var(--font-rune-reading);
}
.folder-row {
  display: flex;
  gap: 8px;
}
.folder-input {
  flex: 1;
  min-width: 0;
  border-radius: var(--radius-rune-tile);
  background: var(--color-rune-raised);
  color: var(--color-rune-ink);
  padding: 8px 12px;
  font: 400 12px/1.4 var(--font-rune-reading);
}
.found-row {
  display: flex;
  align-items: center;
  gap: 12px;
  border-radius: var(--radius-rune-tile);
  background: var(--color-rune-surface);
  padding: 8px 12px;
}
.download-button {
  align-self: flex-start;
}
</style>