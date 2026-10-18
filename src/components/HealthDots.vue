<script setup lang="ts">
import { computed } from 'vue'

/**
 * The model server's health as three tiny dots at the bottom of the orb's face, like one more
 * line of its dot matrix: all lit when fine, two when it needs attention, one in red when it is
 * down, and a travelling light while it is being checked. Red is only ever for trouble.
 */
const props = defineProps<{ tone: 'ok' | 'warn' | 'down' | 'checking' }>()

/** Brightness of each of the three dots (0..1). */
const lit = computed(() => {
  switch (props.tone) {
    case 'ok':
      return [1, 1, 1]
    case 'warn':
      return [1, 1, 0.18]
    case 'down':
      return [1, 0.18, 0.18]
    default:
      return [0.18, 0.18, 0.18]
  }
})
</script>

<template>
  <span
    class="pointer-events-none absolute bottom-1 left-1/2 flex -translate-x-1/2 gap-[3px]"
    aria-hidden="true"
  >
    <span
      v-for="(level, index) in lit"
      :key="index"
      class="dot size-0.5 rounded-full"
      :class="[
        tone === 'down' && index === 0 ? 'bg-[#d71921]' : 'bg-[#111]',
        { checking: tone === 'checking' },
      ]"
      :style="{ opacity: level, animationDelay: `${index * 200}ms` }"
    />
  </span>
</template>

<style scoped>
.dot {
  transition: opacity 300ms ease;
}

.checking {
  animation: health-wave 1s ease-in-out infinite;
}

@keyframes health-wave {
  0%,
  100% {
    opacity: 0.18;
  }
  50% {
    opacity: 1;
  }
}

@media (prefers-reduced-motion: reduce) {
  .checking {
    animation: none;
  }
}
</style>