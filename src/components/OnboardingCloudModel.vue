<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useConnections, type ConnectionEdit } from '@/composables/useConnections'
import type { BackendStatus, Settings } from '@/types/protocol'
import ConnectionEditor from './ConnectionEditor.vue'
import RuneButton from './RuneButton.vue'

const emit = defineEmits<{ ready: []; cancel: [] }>()
const connections = useConnections()
const connectionId = ref('')
const models = ref<NonNullable<BackendStatus['models']>>([])
const model = ref('')
const error = ref('')
const busy = ref(false)
const editing = ref(false)
const current = computed(
  () =>
    connections.catalog.value?.connections.find((item) => item.id === connectionId.value) ?? null,
)
const savedConnection = localStorage.getItem('rune-onboarding-cloud-connection')
watch(
  connections.catalog,
  (catalog) => {
    if (
      !connectionId.value &&
      savedConnection &&
      catalog?.connections.some((item) => item.id === savedConnection)
    ) {
      connectionId.value = savedConnection
      void check()
    }
  },
  { immediate: true },
)
function editConnection() {
  editing.value = true
  error.value = ''
}

async function save(edit: ConnectionEdit) {
  const before = new Set(connections.catalog.value?.connections.map((item) => item.id) ?? [])
  if (!(await connections.save(edit))) return
  connectionId.value =
    edit.id ?? connections.catalog.value?.connections.find((item) => !before.has(item.id))?.id ?? ''
  if (!connectionId.value) {
    error.value = 'The connection was saved but could not be identified. Try again.'
    return
  }
  localStorage.setItem('rune-onboarding-cloud-connection', connectionId.value)
  editing.value = false
  await check()
}
async function check() {
  if (!connectionId.value || !connections.catalog.value) return
  busy.value = true
  error.value = ''
  models.value = []
  try {
    const status = await invoke<BackendStatus>('connections_test', {
      id: connectionId.value,
      revision: connections.catalog.value.revision,
    })
    if (!status.reachable || !status.authOk || status.error)
      throw new Error(status.error || 'The server is not ready.')
    models.value = status.models ?? []
    if (!models.value.length) error.value = 'The server did not report any models.'
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}
async function useModel() {
  const selected = models.value.find((item) => item.id === model.value)
  if (!selected) return
  if (selected.imageIn === 'no' || selected.toolCalls === 'no') {
    error.value = 'Choose a model that supports image input and tool calls.'
    return
  }
  busy.value = true
  error.value = ''
  try {
    if (!(await connections.activate(connectionId.value))) return
    const settings = await invoke<Settings>('get_settings')
    settings.models.agent = model.value
    settings.model = model.value
    await invoke('save_settings', { settings })
    emit('ready')
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="cloud-setup">
    <p>Screen content and task data will be sent to the server you choose.</p>
    <ConnectionEditor
      v-if="!connectionId || editing"
      :connection="current"
      :busy="connections.pending.value || busy"
      @save="save"
      @cancel="connectionId ? (editing = false) : $emit('cancel')"
    />
    <template v-else>
      <p v-if="busy">Checking connection…</p>
      <label v-if="models.length" class="model-label"
        >Agent model
        <select v-model="model" aria-label="Agent model" class="model-select">
          <option value="" disabled>Select a model</option>
          <option v-for="item in models" :key="item.id" :value="item.id">{{ item.id }}</option>
        </select>
      </label>
      <RuneButton v-if="models.length" :disabled="!model || busy" @click="useModel"
        >Use this model</RuneButton
      >
      <RuneButton :disabled="busy" @click="check">Check connection again</RuneButton>
      <RuneButton :disabled="busy" @click="editConnection">Edit connection</RuneButton>
    </template>
    <p v-if="error || connections.error.value" role="alert">
      {{ error || connections.error.value }}
    </p>
  </div>
</template>

<style scoped>
.cloud-setup {
  min-width: 0;
  max-height: 60vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.cloud-setup p {
  margin: 0;
}
.cloud-setup [role='alert'] {
  color: var(--color-error);
}
.model-label {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.model-select {
  color: var(--color-rune-ink);
  background: var(--color-rune-sunken);
  padding: 12px;
}
</style>