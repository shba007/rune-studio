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
  drawable(props.snapshot)
    .map((finding, index) => {
      const capture = props.snapshot.id.startsWith('insight-') ? props.snapshot.monitor : null
      const rect = boxToCss(
        finding.box!,
        capture ? { ...props.view, width: capture.width, height: capture.height } : props.view,
      )
      if (capture) {
        rect.left += (capture.x - (props.view.originX ?? 0)) / props.view.scaleFactor
        rect.top += (capture.y - (props.view.originY ?? 0)) / props.view.scaleFactor
      }
      return {
        n: finding.n,
        label: finding.label,
        rect,
        badge: badgePosition(rect, props.view),
        opacity: markerOpacity(finding.n, props.snapshot.selected),
        delay: index * 40,
      }
    })
    .filter(
      (marker) =>
        marker.rect.left + marker.rect.width > 0 &&
        marker.rect.top + marker.rect.height > 0 &&
        marker.rect.left < props.view.width / props.view.scaleFactor &&
        marker.rect.top < props.view.height / props.view.scaleFactor,
    ),
)
</script>

<template>
  <div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
    <p
      v-if="snapshot.id.startsWith('insight-') && markers.length"
      class="absolute top-6 left-1/2 -translate-x-1/2 rounded-full bg-[#141414]/90 px-4 py-2 text-caption text-zinc-300 uppercase"
    >
      Captured observations · Escape
    </p>
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
      <div
        class="absolute inset-0 rounded-sm bg-[#2d71c3]/10 [background-image:radial-gradient(circle,rgba(45,113,195,.35)_1px,transparent_1px)] [background-size:6px_6px]"
      />
      <span
        v-for="corner in [
          'top-0 left-0 border-t-2 border-l-2',
          'top-0 right-0 border-t-2 border-r-2',
          'bottom-0 left-0 border-b-2 border-l-2',
          'bottom-0 right-0 border-b-2 border-r-2',
        ]"
        :key="corner"
        class="absolute size-3 border-[#f1f1ef] [filter:drop-shadow(0_0_1px_#141414)_drop-shadow(0_1px_1px_#141414)]"
        :class="corner"
      />
      <span
        class="absolute top-0 left-4 max-w-52 -translate-y-full truncate rounded-t-md bg-[#141414]/90 px-2 py-1 text-caption text-[#f1f1ef] uppercase"
        >{{ marker.n }} · {{ marker.label }}</span
      >
      <span
        class="absolute top-1/2 left-1/2 size-1.5 -translate-1/2 rounded-full bg-[#f1f1ef] shadow-[0_0_0_2px_#14141466]"
      />
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