<script setup lang="ts">
import { onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import RuneButton from './RuneButton.vue'

interface Engine {
  name: string
  installed: boolean
  managed: boolean
  version: string
  release_check_available?: boolean
  installation_available?: boolean
  update_available?: boolean
}
interface Installation {
  name: string
  operation?: 'install' | 'update'
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  message: string
}
const props = defineProps<{ connectionId: string; connectionRevision: number }>()
const engines = ref<Engine[]>([])
const busy = ref(false)
const checking = ref<string | null>(null)
const releases = ref<Record<string, { latest: string; detail: string }>>({})
const installation = ref<Installation | null>(null)
const cancelling = ref(false)
let pollTimer: ReturnType<typeof setTimeout> | undefined
let disposed = false
onUnmounted(() => {
  disposed = true
  if (pollTimer) clearTimeout(pollTimer)
})
const status = ref('Load the engine inventory from the active Infer server.')
async function refresh() {
  busy.value = true
  releases.value = {}
  try {
    const result = await invoke<{ engines: Engine[]; installation?: Installation | null }>(
      'infer_engines',
    )
    engines.value = result.engines
    installation.value = result.installation ?? null
    cancelling.value =
      installation.value?.status === 'running' &&
      installation.value.message.startsWith('Cancelling')
    status.value = engines.value.length
      ? 'Inventory provided by Infer.'
      : 'No engines reported by Infer.'
    if (installation.value?.status === 'running') schedulePoll(installation.value.name)
  } catch (error) {
    engines.value = []
    status.value = String(error)
  } finally {
    busy.value = false
  }
}
function schedulePoll(name: string) {
  if (pollTimer) clearTimeout(pollTimer)
  pollTimer = setTimeout(() => void pollInstallation(name), 2000)
}
async function pollInstallation(name: string) {
  try {
    const command =
      installation.value?.operation === 'update'
        ? 'infer_engine_update'
        : 'infer_engine_installation'
    const job = await invoke<Installation>(command, { name, start: false })
    if (disposed) return
    installation.value = job
    if (job.status !== 'running') cancelling.value = false
    status.value = job.message
    if (job.status === 'running') schedulePoll(name)
    else if (job.status === 'completed') {
      await refresh()
      status.value = job.message
    }
  } catch (error) {
    if (!disposed) status.value = `${String(error)}. Load engines to check installation status.`
  }
}
async function runEngineOperation(name: string, operation: 'install' | 'update') {
  checking.value = name
  try {
    const command = operation === 'update' ? 'infer_engine_update' : 'infer_engine_installation'
    const job = await invoke<Installation>(command, { name, start: true })
    if (disposed) return
    installation.value = job
    cancelling.value = false
    status.value = job.message
    schedulePoll(name)
  } catch (error) {
    if (!disposed) status.value = String(error)
  } finally {
    if (!disposed) checking.value = null
  }
}
async function cancelEngineOperation() {
  const job = installation.value
  if (!job || job.status !== 'running' || cancelling.value) return
  cancelling.value = true
  try {
    const updated = await invoke<Installation>('infer_engine_cancel', {
      name: job.name,
      operation: job.operation ?? 'install',
      connectionId: props.connectionId,
      connectionRevision: props.connectionRevision,
    })
    if (disposed) return
    installation.value = updated
    status.value = updated.message
    schedulePoll(job.name)
  } catch (error) {
    if (!disposed) {
      cancelling.value = false
      status.value = String(error)
    }
  }
}
async function checkRelease(name: string) {
  checking.value = name
  try {
    releases.value[name] = await invoke<{ latest: string; detail: string }>(
      'infer_engine_release',
      { name },
    )
    status.value = `Release check completed for ${name}.`
  } catch (error) {
    delete releases.value[name]
    status.value = String(error)
  } finally {
    checking.value = null
  }
}
</script>

<template>
  <HubGroup title="Infer engines">
    <HubRow label="Engine inventory" hint="Installed engines reported by this Infer server.">
      <RuneButton size="sm" :disabled="busy || checking !== null" @click="refresh">{{
        busy ? 'Loading…' : 'Load engines'
      }}</RuneButton>
    </HubRow>
    <HubRow
      v-for="engine in engines"
      :key="engine.name"
      :label="engine.name"
      :hint="
        engine.installed
          ? engine.name === 'encoder'
            ? 'Built in'
            : engine.managed
              ? 'Managed by Infer'
              : 'Configured outside the managed engine package'
          : 'Not installed'
      "
    >
      <span class="text-rune-caption text-rune-text-2">{{ engine.version }}</span>
      <span v-if="releases[engine.name]" class="text-rune-caption text-rune-text-2">{{
        `${releases[engine.name].latest} · ${releases[engine.name].detail}`
      }}</span>
      <RuneButton
        v-if="engine.release_check_available"
        size="sm"
        :disabled="busy || checking !== null"
        @click="checkRelease(engine.name)"
      >
        {{ checking === engine.name ? 'Checking…' : 'Check release' }}
      </RuneButton>
      <RuneButton
        v-if="engine.installation_available"
        size="sm"
        :disabled="busy || checking !== null || installation?.status === 'running'"
        @click="runEngineOperation(engine.name, 'install')"
      >
        {{
          installation?.name === engine.name && installation.status === 'running'
            ? 'Installing…'
            : 'Install'
        }}
      </RuneButton>
      <RuneButton
        v-if="engine.update_available"
        size="sm"
        :disabled="busy || checking !== null || installation?.status === 'running'"
        @click="runEngineOperation(engine.name, 'update')"
      >
        {{
          installation?.name === engine.name &&
          installation.operation === 'update' &&
          installation.status === 'running'
            ? 'Updating…'
            : 'Update'
        }}
      </RuneButton>
      <RuneButton
        v-if="installation?.name === engine.name && installation.status === 'running'"
        size="sm"
        :disabled="cancelling"
        @click="cancelEngineOperation"
      >
        {{ cancelling ? 'Cancelling…' : 'Cancel' }}
      </RuneButton>
    </HubRow>
    <template #after
      ><p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
        {{ status }}
      </p></template
    >
  </HubGroup>
</template>