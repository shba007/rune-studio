<script setup lang="ts">
/**
 * The black pill at the top of the screen, shared by "an agent is in control" and "screen analysis":
 * one shell, so control mode looks exactly as it did and analysis looks like its sibling.
 */
</script>

<template>
  <div
    class="absolute top-6 left-1/2 flex h-10 -translate-x-1/2 items-center gap-3 rounded-full bg-black/85 pr-1 pl-4 text-white"
  >
    <slot />
  </div>
</template>