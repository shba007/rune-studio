<script setup lang="ts">
import RuneButton from './RuneButton.vue'
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
  'flex h-9 cursor-pointer items-center gap-2 rounded-full bg-rune-surface px-4 text-rune-body text-rune-ink transition hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50'
</script>

<template>
  <div class="flex flex-col gap-8">
    <div class="flex items-center gap-4 pb-6">
      <img :src="LOGO" alt="" class="size-12" />
      <div class="flex flex-col">
        <span class="text-rune-title">Rune Studio</span>
        <span class="mt-1 text-rune-body text-rune-text-2"
          >Your goals, supported by local intelligence.</span
        >
      </div>
    </div>

    <HubGroup title="Installed versions">
      <HubRow label="Rune Studio"
        ><span class="text-rune-body text-rune-text-2">{{
          updater.version.value || 'Unavailable'
        }}</span></HubRow
      >
      <HubRow label="Studio CLI"
        ><span class="text-rune-body text-rune-text-2">{{ cliVersion }}</span></HubRow
      >
    </HubGroup>

    <HubGroup title="Updates">
      <HubRow label="Status" :hint="updater.error.value ?? undefined">
        <span class="text-rune-body text-rune-text-2">{{ status() }}</span>
      </HubRow>
      <HubRow label="Check for updates">
        <RuneButton
          size="sm"
          v-if="updater.phase.value === 'available' && updater.latest.value"
          :disabled="agent.isRunning.value"
          @click="updater.install(agent.isRunning.value)"
        >
          <AppIcon name="ok" size="sm" />
          {{
            agent.isRunning.value
              ? 'Finish the task first'
              : `Update to ${updater.latest.value.version}`
          }}
        </RuneButton>
        <RuneButton
          size="sm"
          v-else
          :disabled="updater.phase.value === 'checking' || updater.updating.value"
          @click="updater.check()"
        >
          <AppIcon name="refresh" size="sm" />
          Check for updates
        </RuneButton>
      </HubRow>
    </HubGroup>
    <HubGroup v-if="platform || arch" title="Environment">
      <HubRow v-if="platform" label="Platform"
        ><span class="text-rune-body capitalize text-rune-text-2">{{ platform }}</span></HubRow
      >
      <HubRow v-if="arch" label="Architecture"
        ><span class="text-rune-body text-rune-text-2">{{ arch }}</span></HubRow
      >
    </HubGroup>
    <HubGroup title="Help">
      <HubRow label="Documentation"
        ><RuneButton variant="ghost" size="sm" @click="openLink('docs')">
          Read the guide ↗
        </RuneButton></HubRow
      >
      <HubRow label="Release notes"
        ><RuneButton variant="ghost" size="sm" @click="openLink('releases')">
          What changed ↗
        </RuneButton></HubRow
      >
      <HubRow label="Feedback"
        ><RuneButton variant="ghost" size="sm" @click="openLink('feedback')">
          Report an issue ↗
        </RuneButton></HubRow
      >
    </HubGroup>
    <HubGroup title="License">
      <HubRow label="Rune Studio" hint="Open source under the MIT license."
        ><RuneButton variant="ghost" size="sm" @click="openLink('license')">
          MIT ↗
        </RuneButton></HubRow
      >
    </HubGroup>
    <p v-if="linkError" role="alert" class="m-0 text-rune-body text-error">{{ linkError }}</p>
  </div>
</template>