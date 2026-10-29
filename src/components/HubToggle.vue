<script setup lang="ts">
import HubRow from './HubRow.vue'
import RuneSwitch from './RuneSwitch.vue'

defineProps<{ label: string; hint?: string; info?: string; disabled?: boolean }>()

const model = defineModel<boolean>({ required: true })
</script>

<template>
  <HubRow :label="label" :hint="hint" :info="info">
    <RuneSwitch v-model="model" :label="label" :disabled="disabled" />
  </HubRow>
</template>