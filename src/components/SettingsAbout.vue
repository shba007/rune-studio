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
const platform = ref('')
const arch = ref('')
const linkError = ref('')
async function openLink(target: string) {
  linkError.value = ''
  try {
    await invoke('open_project_link', { target })
  } catch (error) {
    linkError.value = String(error)
  }
}
onMounted(async () => {
  await updater.loadVersion()
  try {
    const versions = await invoke<{ cli: string | null; platform?: string; arch?: string }>(
      'installed_versions',
    )
    cliVersion.value = versions.cli || 'Not available in this installation'
    platform.value = versions.platform ?? ''
    arch.value = versions.arch ?? ''
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
  'flex h-9 cursor-pointer items-center gap-2 rounded-full bg-white/8 px-4 text-label text-zinc-200 transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50'
</script>

<template>
  <div class="flex flex-col gap-8">
    <div class="flex items-center gap-4 border-b border-white/8 pb-6">
      <img :src="LOGO" alt="" class="size-12" />
      <div class="flex flex-col">
        <span class="text-title">Rune Studio</span>
        <span class="mt-1 text-label text-zinc-500"
          >Your goals, supported by local intelligence.</span
        >
      </div>
    </div>

    <HubGroup title="Installed versions">
      <HubRow label="Rune Studio"
        ><span class="text-label text-zinc-400">{{
          updater.version.value || 'Unavailable'
        }}</span></HubRow
      >
      <HubRow label="Studio CLI"
        ><span class="text-label text-zinc-400">{{ cliVersion }}</span></HubRow
      >
    </HubGroup>

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
          Check for updates
        </button>
      </HubRow>
    </HubGroup>
    <HubGroup v-if="platform || arch" title="Environment">
      <HubRow v-if="platform" label="Platform"
        ><span class="text-label capitalize text-zinc-400">{{ platform }}</span></HubRow
      >
      <HubRow v-if="arch" label="Architecture"
        ><span class="text-label text-zinc-400">{{ arch }}</span></HubRow
      >
    </HubGroup>
    <HubGroup title="Help">
      <HubRow label="Documentation"
        ><button
          class="cursor-pointer text-label text-zinc-400 hover:text-white"
          @click="openLink('docs')"
        >
          Read the guide ↗
        </button></HubRow
      >
      <HubRow label="Release notes"
        ><button
          class="cursor-pointer text-label text-zinc-400 hover:text-white"
          @click="openLink('releases')"
        >
          What changed ↗
        </button></HubRow
      >
      <HubRow label="Feedback"
        ><button
          class="cursor-pointer text-label text-zinc-400 hover:text-white"
          @click="openLink('feedback')"
        >
          Report an issue ↗
        </button></HubRow
      >
    </HubGroup>
    <HubGroup title="License">
      <HubRow label="Rune Studio" hint="Open source under the MIT license."
        ><button
          class="cursor-pointer text-label text-zinc-400 hover:text-white"
          @click="openLink('license')"
        >
          MIT ↗
        </button></HubRow
      >
    </HubGroup>
    <p v-if="linkError" role="alert" class="m-0 text-label text-[#ff6b72]">{{ linkError }}</p>
  </div>
</template>