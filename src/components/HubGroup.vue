<script setup lang="ts">
defineProps<{ title?: string; hint?: string }>()
</script>

<template>
  <section class="flex flex-col gap-rune-1">
    <h3 v-if="title" class="m-0 px-rune-2 font-sans text-rune-heading text-rune-text-3 uppercase">
      {{ title }}
    </h3>
    <p v-if="hint" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">{{ hint }}</p>
    <div class="flex min-w-0 flex-col gap-0.5">
      <slot />
    </div>
    <slot name="after" />
  </section>
</template>