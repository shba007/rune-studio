<script setup lang="ts">
defineProps<{ title?: string }>()
</script>

<template>
  <section class="flex flex-col gap-3">
    <h3 v-if="title" class="m-0 text-body font-medium text-zinc-400">{{ title }}</h3>
    <div class="min-w-0">
      <slot />
    </div>
  </section>
</template>