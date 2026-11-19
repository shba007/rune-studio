<script setup lang="ts">
import { computed } from 'vue'

import type { LiveStep } from '@/config/liveSteps'
import { stagedPlugins } from '@/config/stagedChanges'

import StagedChanges from './StagedChanges.vue'

// Surfaces changes a run staged at the level of the run, so nobody has to open the right step
// to find the Apply button. It stays after the run ends, until the change is applied.
const props = defineProps<{ steps: LiveStep[] }>()
const staged = computed(() => stagedPlugins(props.steps))
</script>

<template>
  <section
    v-if="staged.length"
    class="flex min-w-0 flex-col gap-2"
    aria-label="Changes waiting for your OK"
  >
    <h3 class="m-0 text-rune-caption text-rune-text-2 uppercase">Changes waiting for your OK</h3>
    <StagedChanges v-for="item in staged" :key="`${item.plugin}:${item.count}`" :tool="item.tool" />
  </section>
</template>