<script setup lang="ts">
defineProps<{ title: string }>()
</script>

<template>
  <section class="flex flex-col gap-1.5">
    <h3 class="px-1 text-caption text-zinc-500 uppercase">{{ title }}</h3>
    <div class="divide-y divide-white/8 overflow-hidden rounded-2xl bg-white/5">
      <slot />
    </div>
  </section>
</template>