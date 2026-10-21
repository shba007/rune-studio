<script setup lang="ts">
import type { Agent } from '@/composables/useAgent'

defineProps<{ agent: Agent; current: 'settings' | 'artifacts' }>()
</script>

<template>
  <nav class="flex gap-1 px-4 pb-2" aria-label="Pages">
    <button
      v-for="page in ['settings', 'artifacts'] as const"
      :key="page"
      type="button"
      class="h-6 flex-1 cursor-pointer rounded-full border bg-transparent text-caption uppercase transition"
      :class="
        current === page
          ? 'border-(--accent) bg-white/10 text-zinc-50'
          : 'border-white/12 text-zinc-500 hover:text-zinc-200'
      "
      :aria-current="current === page ? 'page' : undefined"
      @click="current !== page && agent.setMode(page)"
    >
      {{ page }}
    </button>
  </nav>
</template>