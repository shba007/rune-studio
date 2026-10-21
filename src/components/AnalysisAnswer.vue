<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{ text: string; waiting: boolean }>()

const expanded = ref(false)
const LONG = 160
const long = computed(() => props.text.length > LONG || props.text.split('\n').length > 4)
</script>

<template>
  <section
    class="rounded-[28px] border border-white/15 bg-[#141414] p-4 text-zinc-100"
    aria-label="Answer"
  >
    <div
      v-if="waiting"
      class="flex flex-col gap-2"
      role="status"
      aria-label="Waiting for the answer"
    >
      <div v-for="row in 3" :key="row" class="flex gap-1" aria-hidden="true">
        <span
          v-for="dot in 14"
          :key="dot"
          class="skeleton-dot size-1 rounded-full bg-[#f1f1ef]"
          :style="{ animationDelay: `${(row + dot) * 70}ms` }"
        />
      </div>
    </div>
    <template v-else>
      <p class="m-0 text-body whitespace-pre-line" :class="expanded ? '' : 'line-clamp-4'">
        {{ text || 'No answer was given.' }}
      </p>
      <button
        v-if="long"
        type="button"
        class="mt-2 cursor-pointer text-caption text-zinc-400 uppercase outline-none hover:text-white focus-visible:ring-2 focus-visible:ring-white/60"
        :aria-expanded="expanded"
        @click="expanded = !expanded"
      >
        {{ expanded ? 'Less' : 'More' }}
      </button>
    </template>
  </section>
</template>

<style scoped>
.skeleton-dot {
  opacity: 0.2;
}

@media (prefers-reduced-motion: no-preference) {
  .skeleton-dot {
    animation: wave 1.2s ease-in-out infinite;
  }
}

@keyframes wave {
  0%,
  100% {
    opacity: 0.2;
  }
  50% {
    opacity: 0.9;
  }
}
</style>