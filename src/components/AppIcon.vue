<script setup lang="ts">
import { icons, type IconName } from '@/icons'

withDefaults(defineProps<{ name: IconName; size?: 'sm' | 'md' }>(), { size: 'md' })

/** Literal class names, so Tailwind can see them. See the icon size roles in src/style.css. */
const SIZE_CLASS = { sm: 'icon-sm', md: 'icon-md' } as const
</script>

<template>
  <component :is="icons[name]" :class="SIZE_CLASS[size]" aria-hidden="true" />
</template>