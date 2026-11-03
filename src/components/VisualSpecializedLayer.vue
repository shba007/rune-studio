<script setup lang="ts">
import { classColor } from '@/config/segmentation'
defineProps<{
  instances: { mask: string; label: string }[]
  oriented: { corners: [number, number][]; label: string }[]
}>()
</script>
<template>
  <div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
    <div
      v-for="(item, index) in instances"
      :key="index"
      class="absolute inset-0 opacity-50"
      :style="{
        backgroundColor: `rgb(${classColor(index).join(' ')})`,
        maskImage: `url('${item.mask}')`,
        maskMode: 'luminance',
        maskSize: '100% 100%',
        maskRepeat: 'no-repeat',
      }"
    />
    <svg class="absolute inset-0 size-full" viewBox="0 0 1000 1000" preserveAspectRatio="none">
      <polygon
        v-for="(item, index) in oriented"
        :key="index"
        :points="item.corners.map((point) => point.join(',')).join(' ')"
        fill="none"
        stroke="#67e8f9"
        stroke-width="2"
        vector-effect="non-scaling-stroke"
      />
    </svg>
  </div>
</template>