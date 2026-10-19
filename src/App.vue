<script setup lang="ts">
import { onMounted } from 'vue'

import AgentPill from '@/components/AgentPill.vue'
import { useUpdater } from '@/composables/useUpdater'

// Owns the update lifecycle: the version is read and one quiet check is made shortly after launch.
const updater = useUpdater()
onMounted(updater.startup)
</script>

<template>
  <AgentPill />
</template>