<script setup lang="ts">
import type { Settings } from '@/types/protocol'

import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'

defineProps<{ draft: Settings }>()
</script>

<template>
  <HubGroup>
    <HubInput v-model="draft.toggleHotkey" label="Open and close" />
    <HubInput v-model="draft.stopHotkey" label="Stop" />
  </HubGroup>
</template>