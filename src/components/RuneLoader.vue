<template>
  <span class="loader-grid" aria-hidden="true"><span v-for="n in 4" :key="n" /></span>
</template>

<style scoped>
.loader-grid {
  display: inline-grid;
  grid-template-columns: repeat(2, 5px);
  gap: 3px;
  flex-shrink: 0;
}
.loader-grid span {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--color-rune-text-2);
  animation: pulse 1.2s ease-in-out infinite;
}
.loader-grid span:nth-child(2) {
  animation-delay: 0.15s;
}
.loader-grid span:nth-child(4) {
  animation-delay: 0.3s;
}
.loader-grid span:nth-child(3) {
  animation-delay: 0.45s;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 0.25;
  }
  50% {
    opacity: 1;
  }
}
@media (prefers-reduced-motion: reduce) {
  .loader-grid span {
    animation: none;
    opacity: 0.7;
  }
}
</style>