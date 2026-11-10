<script setup lang="ts">
import { ref, watch } from 'vue'

import { loadMcpIcon } from '@/config/mcpIcon'

import AppIcon from './AppIcon.vue'

const props = defineProps<{ name: string }>()
const src = ref<string | null>(null)

watch(
  () => props.name,
  async (name) => {
    src.value = null
    const icon = await loadMcpIcon(name)
    if (name === props.name) src.value = icon
  },
  { immediate: true },
)
</script>
<template>
  <span
    v-if="src"
    aria-hidden="true"
    class="inline-block size-4 shrink-0 bg-current align-middle"
    :style="{
      maskImage: `url('${src}')`,
      maskSize: 'contain',
      maskRepeat: 'no-repeat',
      maskPosition: 'center',
    }"
  />
  <AppIcon v-else name="plug" size="sm" class="shrink-0" />
</template>