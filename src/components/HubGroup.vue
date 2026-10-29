<script setup lang="ts">
defineProps<{ title?: string }>()
</script>

<template>
  <section class="flex flex-col gap-rune-2">
    <h3 v-if="title" class="m-0 font-sans text-rune-subtitle font-medium text-rune-text-2">
      {{ title }}
    </h3>
    <div class="min-w-0">
      <slot />
    </div>
  </section>
</template>