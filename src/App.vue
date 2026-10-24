<script setup lang="ts">
import { onMounted } from 'vue'

import AgentPill from '@/components/AgentPill.vue'
import { useUpdater } from '@/composables/useUpdater'
import { installCursorAssets } from '@/config/cursor'

const updater = useUpdater()
onMounted(updater.startup)
onMounted(
  () =>
    void installCursorAssets().catch((error) =>
      console.warn('Could not load cursor assets', error),
    ),
)
</script>

<template>
  <AgentPill />
</template>