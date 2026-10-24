<script setup lang="ts">
withDefaults(defineProps<{ kind?: 'default' | 'text'; ripple?: boolean }>(), {
  kind: 'default',
  ripple: false,
})
</script>
<template>
  <span class="pointer-events-none relative block size-8" aria-hidden="true">
    <img :src="`/cursor/${kind}.svg`" alt="" class="block size-full" />
    <span
      v-if="ripple"
      class="cursor-ripple absolute inset-0 overflow-hidden"
      :style="{ maskImage: `url(/cursor/${kind}.svg)`, maskSize: 'contain' }"
      data-testid="ripple"
    >
      <svg viewBox="0 0 32 32" class="size-full">
        <circle
          cx="16"
          cy="16"
          r="10"
          fill="none"
          stroke="white"
          stroke-width="1.5"
          stroke-dasharray="1 3"
          class="cursor-wave"
        />
      </svg>
    </span>
  </span>
</template>
<style scoped>
.cursor-wave {
  transform-origin: center;
}
@media (prefers-reduced-motion: no-preference) {
  .cursor-wave {
    animation: cursor-wave 900ms ease-out infinite;
  }
}
@keyframes cursor-wave {
  from {
    transform: scale(0.15);
    opacity: 1;
  }
  to {
    transform: scale(1.25);
    opacity: 0;
  }
}
</style>