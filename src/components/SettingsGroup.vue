<script setup lang="ts">
defineProps<{ title: string }>()
</script>

<template>
  <section class="flex flex-col gap-2">
    <h3 class="px-2 text-caption text-zinc-500 uppercase">{{ title }}</h3>
    <div class="divide-y divide-white/8 overflow-hidden rounded-2xl bg-white/5">
      <slot />
    </div>
  </section>
</template>