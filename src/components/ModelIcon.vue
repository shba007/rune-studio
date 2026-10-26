<script setup lang="ts">
import { modelIcon } from '@/config/modelIcon'
defineProps<{ model: string }>()
</script>
<template>
  <span
    aria-hidden="true"
    class="inline-block size-4 shrink-0 bg-current align-middle"
    :style="{
      maskImage: `url('${modelIcon(model)}')`,
      maskSize: 'contain',
      maskRepeat: 'no-repeat',
      maskPosition: 'center',
    }"
  />
</template>