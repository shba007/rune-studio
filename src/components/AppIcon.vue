<script setup lang="ts">
import { icons, type IconName } from '@/icons'

withDefaults(defineProps<{ name: IconName; size?: 'sm' | 'md' }>(), { size: 'md' })

const SIZE_CLASS = { sm: 'icon-sm', md: 'icon-md' } as const
</script>

<template>
  <component :is="icons[name]" :class="SIZE_CLASS[size]" aria-hidden="true" />
</template>