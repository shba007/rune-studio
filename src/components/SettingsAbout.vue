<script setup lang="ts">
import type { Agent } from '@/composables/useAgent'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useUpdater } from '@/composables/useUpdater'

import AppIcon from './AppIcon.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'

defineProps<{ agent: Agent }>()

const updater = useUpdater()
const cliVersion = ref('Loading…')
onMounted(async () => {
  await updater.loadVersion()
  try {
    const versions = await invoke<{ cli: string | null }>('installed_versions')
    cliVersion.value = versions.cli || 'Not available in this installation'
  } catch {
    cliVersion.value = 'Unavailable'
  }
})

const LOGO = '/logo.svg'

function status() {
  if (updater.phase.value === 'checking') return 'Checking...'
  if (updater.phase.value === 'uptodate') return 'Up to date'
  if (updater.phase.value === 'available' && updater.latest.value)
    return `Version ${updater.latest.value.version} is available`
  if (updater.phase.value === 'error') return 'Could not check for updates'
  if (updater.updating.value) return 'Updating...'
  return ''
}

const button =
  'flex h-9 cursor-pointer items-center gap-2 rounded-full border border-white/25 bg-transparent px-4 text-label text-zinc-200 uppercase transition hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-50'
</script>

<template>
  <div class="flex flex-col gap-8">
    <div class="flex items-center gap-4">
      <img :src="LOGO" alt="" class="size-14" />
      <div class="flex flex-col">
        <span class="text-title">Rune Studio</span>
        <span class="text-label text-zinc-400"
          >Installed Studio {{ updater.version.value || 'Unavailable' }}</span
        >
        <span class="text-label text-zinc-500">CLI {{ cliVersion }}</span>
      </div>
    </div>

    <HubGroup title="Updates">
      <HubRow label="Status" :hint="updater.error.value ?? undefined">
        <span class="text-label text-zinc-300">{{ status() }}</span>
      </HubRow>
      <HubRow label="Check for updates">
        <button
          v-if="updater.phase.value === 'available' && updater.latest.value"
          type="button"
          :class="button"
          :disabled="agent.isRunning.value"
          @click="updater.install(agent.isRunning.value)"
        >
          <AppIcon name="ok" size="sm" />
          {{
            agent.isRunning.value
              ? 'Finish the task first'
              : `Update to ${updater.latest.value.version}`
          }}
        </button>
        <button
          v-else
          type="button"
          :class="button"
          :disabled="updater.phase.value === 'checking' || updater.updating.value"
          @click="updater.check()"
        >
          <AppIcon name="refresh" size="sm" />
          Check
        </button>
      </HubRow>
    </HubGroup>
  </div>
</template>