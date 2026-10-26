<script setup lang="ts">
import { usePreferredReducedMotion } from '@vueuse/core'
import { computed } from 'vue'

import { boxToCss, drawable, markerOpacity, readPose, type OverlayView } from '@/config/analysis'
import type { AnalysisSnapshot } from '@/types/protocol'
import { annotationLayout } from '@/config/annotations'

const props = defineProps<{ snapshot: AnalysisSnapshot; view: OverlayView; scanKey: number }>()

const reduced = usePreferredReducedMotion()
const markers = computed(() =>
  drawable(props.snapshot)
    .map((finding, index) => {
      const pose = finding.group === 'structure' ? readPose(finding.pose) : null
      const points = pose?.keypoints ?? []
      const bounds = finding.box ?? [
        Math.min(...points.map((p) => p.x)),
        Math.min(...points.map((p) => p.y)),
        Math.max(...points.map((p) => p.x)),
        Math.max(...points.map((p) => p.y)),
      ]
      const capture = props.snapshot.id.startsWith('insight-') ? props.snapshot.monitor : null
      const view = capture
        ? { ...props.view, width: capture.width, height: capture.height }
        : props.view
      const rect = boxToCss(bounds as [number, number, number, number], view)
      const keypoints = points.map((p) => ({
        ...p,
        x: (((p.x - bounds[0]!) / 1000) * view.width) / view.scaleFactor,
        y: (((p.y - bounds[1]!) / 1000) * view.height) / view.scaleFactor,
      }))
      if (capture) {
        rect.left += (capture.x - (props.view.originX ?? 0)) / props.view.scaleFactor
        rect.top += (capture.y - (props.view.originY ?? 0)) / props.view.scaleFactor
      }
      return {
        n: finding.n,
        label: finding.label,
        rect,
        pose: pose ? { keypoints, connections: pose.connections } : null,
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
const annotations = computed(() =>
  annotationLayout(
    markers.value.map((marker) => marker.rect),
    props.view.width / props.view.scaleFactor,
    props.view.height / props.view.scaleFactor,
    markers.value.map((marker) => `${marker.n} · ${marker.label}`),
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
        v-if="!marker.pose"
        class="absolute inset-0 rounded-sm bg-[#2d71c3]/10 [background-image:radial-gradient(circle,rgba(45,113,195,.35)_1px,transparent_1px)] [background-size:6px_6px]"
      />
      <svg
        v-if="marker.pose"
        class="absolute top-0 left-0 h-full w-full overflow-visible"
        :data-pose="marker.n"
      >
        <line
          v-for="([a, b], index) in marker.pose.connections"
          :key="index"
          :x1="marker.pose.keypoints[a]!.x"
          :y1="marker.pose.keypoints[a]!.y"
          :x2="marker.pose.keypoints[b]!.x"
          :y2="marker.pose.keypoints[b]!.y"
          stroke="#67e8f9"
          stroke-width="2"
          data-pose-edge=""
        />
        <circle
          v-for="(point, index) in marker.pose.keypoints"
          :key="index"
          :cx="point.x"
          :cy="point.y"
          r="4"
          fill="#f1f1ef"
          stroke="#141414"
          :data-keypoint="point.label"
        >
          <title>{{ point.label }}</title>
        </circle>
      </svg>
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
      <span class="sr-only">{{ marker.n }} · {{ marker.label }}</span>
    </div>
    <svg class="absolute inset-0 h-full w-full overflow-hidden" aria-hidden="true">
      <template v-for="(annotation, index) in annotations" :key="markers[index]!.n">
        <polyline
          v-if="annotation"
          :points="annotation.leader"
          fill="none"
          stroke="#f1f1ef"
          stroke-width="1"
          :opacity="markers[index]!.opacity"
        />
        <rect
          v-if="annotation"
          :x="annotation.anchor.x - 3"
          :y="annotation.anchor.y - 3"
          width="6"
          height="6"
          fill="#f1f1ef"
          :opacity="markers[index]!.opacity"
          data-annotation-endpoint=""
        />
      </template>
    </svg>
    <template v-for="(annotation, index) in annotations" :key="`label-${markers[index]!.n}`">
      <span
        v-if="annotation"
        class="absolute truncate rounded-sm bg-[#141414]/80 px-1.5 text-[10px] font-semibold leading-[18px] tracking-wide text-[#f1f1ef] uppercase"
        :data-annotation="markers[index]!.n"
        :style="{
          left: `${annotation.label.left}px`,
          top: `${annotation.label.top}px`,
          width: `${annotation.label.width}px`,
          height: `${annotation.label.height}px`,
          opacity: markers[index]!.opacity,
        }"
      >
        {{ markers[index]!.n }} · {{ markers[index]!.label }}
      </span>
    </template>
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
</style>