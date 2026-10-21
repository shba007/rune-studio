<script setup lang="ts">
defineProps<{ title?: string }>()
</script>

<template>
  <section class="flex flex-col gap-2">
    <h3 v-if="title" class="m-0 text-caption text-zinc-500 uppercase">{{ title }}</h3>
    <div class="rounded-2xl bg-white/5 px-5">
      <slot />
    </div>
  </section>
</template>