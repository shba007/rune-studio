<script setup lang="ts">
import { ref, watch } from 'vue'

import { GENERIC_MODEL_ICON, loadModelIcon } from '@/config/modelIcon'

const props = defineProps<{ model: string }>()
const src = ref(GENERIC_MODEL_ICON)

watch(
  () => props.model,
  async (model) => {
    src.value = GENERIC_MODEL_ICON
    const icon = await loadModelIcon(model)
    if (model === props.model) src.value = icon
  },
  { immediate: true },
)
</script>
<template>
  <span
    aria-hidden="true"
    class="inline-block size-4 shrink-0 bg-current align-middle"
    :style="{
      maskImage: `url('${src}')`,
      maskSize: 'contain',
      maskRepeat: 'no-repeat',
      maskPosition: 'center',
    }"
  />
</template>