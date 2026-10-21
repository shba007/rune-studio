<script setup lang="ts">
import { useWhen } from '@/composables/useWhen'

const props = defineProps<{ iso: string | null | undefined; relative?: boolean }>()
const { label, ago, full, valid } = useWhen(() => props.iso)
</script>

<template>
  <time v-if="valid" :datetime="iso ?? undefined" :title="full">{{ relative ? ago : label }}</time>
</template>