<script setup lang="ts">
import { onMounted } from 'vue'

import AgentPill from '@/components/AgentPill.vue'
import { useUpdater } from '@/composables/useUpdater'

const updater = useUpdater()
onMounted(updater.startup)
</script>

<template>
  <AgentPill />
</template>