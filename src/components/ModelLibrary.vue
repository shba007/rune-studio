<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import DownloadProgress from './DownloadProgress.vue'
import ModelIcon from './ModelIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneSwitch from './RuneSwitch.vue'

type Candidate = {
  id: string
  name: string
  license: string
  bytes: number
  availableBytes?: number | null
  revision: string
  manifest: string
  recommended?: boolean
}
type DownloadStatus = {
  phase: string
  model?: string
  filename?: string
  downloaded: number
  total?: number
  error?: string
}
type Installed = { id: string; name: string; path: string }
type Found = { id: string; name: string; path: string }

const props = defineProps<{
  downloads: boolean
  connectionId: string
  connectionRevision: number
  busy: boolean
  statuses?: Record<string, string | null | undefined>
}>()
const emit = defineEmits<{ used: [] }>()

const tab = ref<'discover' | 'device'>(props.downloads ? 'discover' : 'device')
const search = ref('')
const candidates = ref<Candidate[]>([])
const installed = ref<Installed[]>([])
const agentModel = ref<string | null>(null)
const download = ref<DownloadStatus | null>(null)
const loading = ref(false)
const starting = ref(false)
const error = ref('')
const unavailable = ref('')
const removing = ref<string | null>(null)
const switching = ref('')
const folder = ref('')
const found = ref<Found[]>([])
const scanning = ref(false)
const scanned = ref(false)
const embedded = computed(() => props.connectionId === 'studio-embedded-infer')
watch(
  folder,
  () => {
    found.value = []
    scanned.value = false
  },
  { flush: 'sync' },
)
let timer: ReturnType<typeof setInterval> | undefined
let disposed = false

const downloading = computed(() =>
  ['preparing', 'downloading', 'engine'].includes(download.value?.phase ?? ''),
)
const installedIds = computed(() => new Set(installed.value.map((model) => model.id)))
const query = computed(() => search.value.trim().toLowerCase())
const discover = computed(() =>
  candidates.value.filter((model) =>
    `${model.name} ${model.id}`.toLowerCase().includes(query.value),
  ),
)
const onDevice = computed(() =>
  installed.value.filter((model) =>
    `${model.name} ${model.id}`.toLowerCase().includes(query.value),
  ),
)
const gigabytes = (bytes: number) => `${(bytes / 1_000_000_000).toFixed(2)} GB`

async function loadInstalled() {
  unavailable.value = ''
  try {
    const document = await invoke<{ catalog: string }>('infer_catalog_read')
    const catalog = JSON.parse(document.catalog) as {
      models?: Array<Record<string, unknown> & { id: string; name?: string }>
    }
    installed.value = (catalog.models ?? [])
      .filter((model) => model.local && typeof model.local === 'object')
      .map((model) => ({
        id: model.id,
        name: model.name || model.id,
        path: String((model.local as Record<string, unknown>).path ?? ''),
      }))
  } catch (e) {
    installed.value = []
    unavailable.value = String(e)
  }
  try {
    agentModel.value = await invoke<string | null>('infer_agent_role')
  } catch {
    agentModel.value = null
  }
}
async function loadCandidates() {
  if (!props.downloads) return
  try {
    const list = await invoke<Candidate[]>('infer_library_candidates', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
    })
    candidates.value = Array.isArray(list) ? list : []
  } catch (e) {
    error.value = String(e)
  }
}
async function refreshDownload() {
  try {
    download.value = await invoke<DownloadStatus>('infer_library_download_status', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
    })
    if (!downloading.value && timer) {
      clearInterval(timer)
      timer = undefined
      if (download.value?.phase === 'complete') await loadInstalled()
    }
  } catch (e) {
    error.value = String(e)
    if (timer) clearInterval(timer)
    timer = undefined
  }
}
async function install(model: Candidate) {
  error.value = ''
  try {
    await invoke('infer_library_download', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
      id: model.id,
      revision: model.revision,
      manifest: model.manifest,
    })
    await refreshDownload()
    if (downloading.value && !timer) timer = setInterval(() => void refreshDownload(), 1000)
  } catch (e) {
    error.value = String(e)
  }
}
async function scanFolder() {
  const directory = folder.value.trim()
  if (!directory || scanning.value) return
  error.value = ''
  scanning.value = true
  scanned.value = false
  found.value = []
  try {
    const models = await invoke<Found[]>('infer_library_scan', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
      directory,
    })
    if (!disposed && folder.value.trim() === directory) {
      found.value = Array.isArray(models) ? models : []
      scanned.value = true
    }
  } catch (e) {
    if (!disposed) error.value = String(e)
  } finally {
    scanning.value = false
  }
}
async function browseFolder() {
  try {
    const picked = await invoke<string | null>('pick_folder')
    if (picked) {
      folder.value = picked
      await scanFolder()
    }
  } catch (e) {
    error.value = String(e)
  }
}
async function importFound(model: Found) {
  const candidate = candidates.value.find((item) => item.id === model.id)
  if (!candidate) {
    error.value = 'This model is no longer available. Reload the model library.'
    return
  }
  error.value = ''
  starting.value = true
  try {
    await invoke('infer_library_download', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
      id: candidate.id,
      revision: candidate.revision,
      manifest: candidate.manifest,
      directory: folder.value.trim(),
    })
    await refreshDownload()
    if (downloading.value && !timer) timer = setInterval(() => void refreshDownload(), 1000)
  } catch (e) {
    error.value = String(e)
  } finally {
    starting.value = false
  }
}
async function cancel() {
  try {
    await invoke('infer_library_download_cancel', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
    })
  } catch (e) {
    error.value = String(e)
  }
}
async function useDownloaded() {
  if (!download.value?.model) return
  starting.value = true
  error.value = ''
  try {
    await invoke('infer_library_select_agent', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
      model: download.value.model,
    })
    emit('used')
    await loadInstalled()
  } catch (e) {
    error.value = String(e)
  } finally {
    starting.value = false
  }
}
async function useInstalled(model: Installed) {
  starting.value = true
  error.value = ''
  try {
    await invoke('infer_library_select_agent', {
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
      model: model.id,
    })
    agentModel.value = model.id
    emit('used')
  } catch (e) {
    error.value = String(e)
  } finally {
    starting.value = false
  }
}
async function setLoaded(model: Installed, load: boolean) {
  switching.value = model.id
  error.value = ''
  try {
    await invoke('infer_model_residency', { model: model.id, load })
    emit('used')
  } catch (e) {
    error.value = String(e)
  } finally {
    switching.value = ''
  }
}
async function removeInstalled(model: Installed) {
  starting.value = true
  error.value = ''
  try {
    const failed = await invoke<string[]>('infer_model_remove', { model: model.id })
    removing.value = null
    await loadInstalled()
    if (failed?.length) error.value = `Some files could not be deleted: ${failed.join(', ')}`
    emit('used')
  } catch (e) {
    error.value = String(e)
  } finally {
    starting.value = false
  }
}
onMounted(async () => {
  loading.value = true
  await Promise.all([loadInstalled(), loadCandidates()])
  if (props.downloads) {
    await refreshDownload()
    if (downloading.value && !timer) timer = setInterval(() => void refreshDownload(), 1000)
  }
  if (!disposed) loading.value = false
})
onBeforeUnmount(() => {
  disposed = true
  if (timer) clearInterval(timer)
})
</script>

<template>
  <section class="flex flex-col gap-rune-1" aria-label="Models">
    <div class="flex flex-wrap items-center justify-between gap-rune-2 px-rune-2">
      <div role="tablist" aria-label="Model library" class="flex gap-rune-1">
        <RuneButton
          v-if="downloads"
          role="tab"
          size="sm"
          :variant="tab === 'discover' ? 'primary' : 'ghost'"
          :aria-selected="tab === 'discover'"
          @click="tab = 'discover'"
          >Discover</RuneButton
        >
        <RuneButton
          role="tab"
          size="sm"
          :variant="tab === 'device' ? 'primary' : 'ghost'"
          :aria-selected="tab === 'device'"
          @click="tab = 'device'"
          >Installed</RuneButton
        >
      </div>
      <span v-if="candidates[0]?.availableBytes != null" class="text-rune-caption text-rune-text-3">
        {{ gigabytes(candidates[0].availableBytes) }} free
      </span>
    </div>
    <input
      v-model="search"
      type="search"
      aria-label="Search models"
      placeholder="Search models"
      class="h-10 w-full rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
    />
    <p v-if="loading" role="status" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
      Loading models…
    </p>

    <template v-else-if="tab === 'discover'">
      <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
        Reviewed models that run on the Infer server. Download sizes exclude the engine and
        temporary files. Check each model's license before use.
      </p>
      <p
        v-if="!candidates.length"
        role="status"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
      >
        No compatible models are available from this Infer server.
      </p>
      <p
        v-else-if="!discover.length"
        role="status"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
      >
        No matching models.
      </p>
      <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
        <li
          v-for="model in discover"
          :key="model.id"
          class="flex items-center gap-rune-2 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
        >
          <ModelIcon :model="model.id" class="size-6" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-rune-body text-rune-ink">
              {{ model.name }}
              <span v-if="model.recommended" class="ml-1 text-rune-caption text-rune-text-3"
                >Recommended</span
              >
            </span>
            <span class="block text-rune-caption text-rune-text-3">
              {{ gigabytes(model.bytes) }} · {{ model.license }}
            </span>
          </span>
          <span v-if="installedIds.has(model.id)" class="text-rune-caption text-rune-text-3"
            >Installed</span
          >
          <RuneButton
            v-else
            size="sm"
            :disabled="downloading || busy"
            :aria-label="`Download ${model.name}`"
            @click="install(model)"
            >Download</RuneButton
          >
        </li>
      </ul>
      <div class="flex flex-col gap-rune-1 px-rune-2 py-rune-2">
        <p class="m-0 text-rune-body text-rune-ink">Import an existing model</p>
        <p class="m-0 text-rune-caption text-rune-text-3">
          Enter a folder path on the active Infer server.
        </p>
        <div class="flex flex-wrap gap-rune-1">
          <input
            v-model="folder"
            type="text"
            aria-label="Infer model folder"
            placeholder="Folder on Infer server"
            class="min-w-0 flex-1 rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-body text-rune-ink"
            :disabled="downloading || starting || busy"
            @keydown.enter.prevent="scanFolder"
          />
          <RuneButton
            v-if="embedded"
            size="sm"
            :disabled="downloading || starting || busy"
            @click="browseFolder"
            >Browse</RuneButton
          >
          <RuneButton
            size="sm"
            :disabled="!folder.trim() || scanning || downloading || starting || busy"
            @click="scanFolder"
            >{{ scanning ? 'Scanning…' : 'Scan' }}</RuneButton
          >
        </div>
        <p
          v-if="scanned && !found.length"
          role="status"
          class="m-0 text-rune-caption text-rune-text-3"
        >
          No supported model was found in this folder.
        </p>
        <div v-for="model in found" :key="model.id" class="flex items-center gap-rune-2">
          <span class="min-w-0 flex-1 text-rune-body text-rune-ink"
            >{{ model.name }}
            <span class="block truncate text-rune-caption text-rune-text-3">{{
              model.path
            }}</span></span
          >
          <RuneButton
            size="sm"
            :disabled="downloading || starting || busy"
            @click="importFound(model)"
            >Import</RuneButton
          >
        </div>
      </div>
      <div
        v-if="download?.phase && download.phase !== 'idle'"
        role="status"
        class="flex flex-wrap items-center gap-rune-2 px-rune-2 text-rune-caption text-rune-text-2"
      >
        <DownloadProgress v-if="downloading" v-bind="download" />
        <span v-else-if="download.phase === 'complete'">Downloaded {{ download.model }}.</span>
        <span v-else-if="download.phase === 'cancelled'">Download cancelled. You can retry.</span>
        <span v-else>{{ download.error || 'Download failed.' }}</span>
        <RuneButton v-if="downloading" size="sm" @click="cancel">Cancel</RuneButton>
        <RuneButton
          v-if="download.phase === 'complete'"
          size="sm"
          variant="primary"
          :disabled="starting || busy"
          @click="useDownloaded"
          >Use this model</RuneButton
        >
      </div>
    </template>

    <template v-else>
      <p v-if="unavailable" role="status" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
        Installed models are unavailable. Check the active connection and its API key.
      </p>
      <p
        v-else-if="!installed.length"
        role="status"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
      >
        No local models are installed yet.{{ downloads ? ' Open Discover to download one.' : '' }}
      </p>
      <p
        v-else-if="!onDevice.length"
        role="status"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
      >
        No matching models.
      </p>
      <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
        <li
          v-for="model in onDevice"
          :key="model.id"
          class="flex flex-wrap items-center gap-rune-2 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
        >
          <ModelIcon :model="model.id" class="size-6" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-rune-body text-rune-ink">{{ model.name }}</span>
            <span class="block truncate text-rune-caption text-rune-text-3">{{ model.id }}</span>
          </span>
          <span v-if="agentModel === model.id" class="text-rune-caption text-rune-text-3"
            >Agent</span
          >
          <RuneButton
            v-else
            size="sm"
            :disabled="starting || busy"
            :aria-label="`Use ${model.name}`"
            @click="useInstalled(model)"
            >Use</RuneButton
          >
          <span class="text-rune-caption text-rune-text-3">{{
            switching === model.id
              ? 'Working…'
              : statuses?.[model.id] === 'loading'
                ? 'Loading…'
                : statuses?.[model.id] === 'loaded'
                  ? 'In memory'
                  : ''
          }}</span>
          <RuneSwitch
            v-if="statuses && statuses[model.id] !== undefined"
            :model-value="statuses[model.id] === 'loaded'"
            :label="`Load ${model.name} into memory`"
            :disabled="starting || busy || !!switching || statuses[model.id] === 'loading'"
            @update:model-value="setLoaded(model, $event)"
          />
          <RuneButton
            v-if="removing !== model.id"
            variant="ghost"
            size="sm"
            :disabled="starting || busy || agentModel === model.id"
            :aria-label="`Remove ${model.name}`"
            @click="removing = model.id"
            >Remove</RuneButton
          >
          <div
            v-else
            class="flex w-full flex-wrap items-center gap-rune-2 rounded-rune-tile bg-rune-sunken p-rune-2 text-rune-caption text-rune-text-2"
          >
            <span class="flex-1">Delete {{ model.name }} and its files from this computer?</span>
            <RuneButton
              variant="danger"
              size="sm"
              :disabled="starting"
              @click="removeInstalled(model)"
              >Confirm remove</RuneButton
            >
            <RuneButton variant="ghost" size="sm" @click="removing = null">Cancel</RuneButton>
          </div>
        </li>
      </ul>
    </template>
    <p v-if="error" role="alert" class="m-0 px-rune-2 text-rune-body text-attention">
      {{ error }}
    </p>
  </section>
</template>