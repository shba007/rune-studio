<script setup lang="ts">
import { usePreferredReducedMotion } from '@vueuse/core'
import { computed } from 'vue'

import {
  badgePosition,
  boxToCss,
  drawable,
  markerOpacity,
  type OverlayView,
} from '@/config/analysis'
import type { AnalysisSnapshot } from '@/types/protocol'

const props = defineProps<{ snapshot: AnalysisSnapshot; view: OverlayView; scanKey: number }>()

const reduced = usePreferredReducedMotion()
const markers = computed(() =>
  drawable(props.snapshot).map((finding, index) => {
    const rect = boxToCss(finding.box!, props.view)
    return {
      n: finding.n,
      rect,
      badge: badgePosition(rect, props.view),
      opacity: markerOpacity(finding.n, props.snapshot.selected),
      delay: index * 40,
    }
  }),
)
</script>

<template>
  <div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
    <div
      v-if="scanKey > 0 && reduced !== 'reduce'"
      :key="scanKey"
      class="scan-line absolute inset-0"
    />
    <div
      v-for="marker in markers"
      :key="`${snapshot.id}-${marker.n}`"
      class="marker absolute"
      :class="reduced === 'reduce' ? '' : 'marker-in'"
      :style="{
        left: `${marker.rect.left}px`,
        top: `${marker.rect.top}px`,
        width: `${marker.rect.width}px`,
        height: `${marker.rect.height}px`,
        opacity: marker.opacity,
        animationDelay: `${marker.delay}ms`,
      }"
      :data-marker="marker.n"
    >
      <div class="absolute inset-0 border-[3px] border-[#141414]/55" />
      <div class="absolute inset-0 border-2 border-dashed border-[#f1f1ef]" />
    </div>
    <span
      v-for="marker in markers"
      :key="`badge-${snapshot.id}-${marker.n}-${snapshot.selected === marker.n}`"
      class="absolute grid size-5 place-items-center rounded-full border border-[#141414] bg-[#f1f1ef] text-caption text-[#141414]"
      :class="[
        snapshot.selected === marker.n && reduced !== 'reduce' ? 'badge-pulse' : '',
        reduced === 'reduce' ? '' : 'marker-in',
      ]"
      :style="{
        left: `${marker.badge.x}px`,
        top: `${marker.badge.y}px`,
        opacity: marker.opacity,
        animationDelay: `${marker.delay}ms`,
      }"
      :data-badge="marker.n"
    >
      {{ marker.n }}
    </span>
  </div>
</template>

<style scoped>
.scan-line {
  background-image: radial-gradient(circle, rgb(241 241 239 / 0.6) 1.2px, transparent 1.6px);
  background-size: 8px 8px;
  animation: sweep 1.8s ease-out forwards;
  mask-image: radial-gradient(
    circle at center,
    transparent calc(var(--scan-radius) - 32px),
    black var(--scan-radius),
    transparent calc(var(--scan-radius) + 32px)
  );
}

@property --scan-radius {
  syntax: '<length>';
  inherits: false;
  initial-value: 0px;
}

.marker-in {
  animation: fade-in 160ms ease-out backwards;
}

.badge-pulse {
  animation: pulse 360ms ease-out;
}

@keyframes sweep {
  from {
    --scan-radius: 0px;
    opacity: 1;
  }
  to {
    --scan-radius: 120vmax;
    opacity: 0;
  }
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
}

@keyframes pulse {
  40% {
    transform: scale(1.35);
  }
}
</style>