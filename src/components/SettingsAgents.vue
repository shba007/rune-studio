<script setup lang="ts">
import { useClipboard } from '@vueuse/core'

import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'

defineProps<{ agent: Agent; draft: Settings }>()

const { copy, copied } = useClipboard({ copiedDuring: 1600 })
</script>

<template>
  <HubGroup>
    <HubToggle v-model="draft.externalAgent" label="Allow outside agent" />
    <HubRow
      v-if="draft.externalAgent && agent.bridge.value?.available"
      label="Connect command"
      :hint="agent.bridge.value.url"
    >
      <button
        type="button"
        class="flex h-9 cursor-pointer items-center gap-2 rounded-full border border-white/25 bg-transparent px-4 text-label text-zinc-200 uppercase transition hover:bg-white/10"
        @click="copy(agent.bridge.value.command)"
      >
        <AppIcon name="copy" size="sm" />
        {{ copied ? 'Copied' : 'Copy' }}
      </button>
    </HubRow>
  </HubGroup>
</template>