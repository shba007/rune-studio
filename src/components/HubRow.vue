<script setup lang="ts">
import { ref } from 'vue'

import AppIcon from './AppIcon.vue'
import RuneIconButton from './RuneIconButton.vue'

defineProps<{ label: string; hint?: string; info?: string }>()

const showInfo = ref(false)
</script>

<template>
  <div
    class="flex min-h-16 flex-col justify-center bg-rune-surface px-rune-2 py-rune-1 first:rounded-t-rune-tile last:rounded-b-rune-tile"
  >
    <div class="flex flex-wrap items-center gap-x-8 gap-y-rune-1">
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="flex items-center gap-2 text-rune-body text-rune-ink">
          {{ label }}
          <RuneIconButton
            v-if="info"
            size="xs"
            variant="ghost"
            :label="showInfo ? `Hide details: ${label}` : `Details: ${label}`"
            :aria-expanded="showInfo"
            @click="showInfo = !showInfo"
          >
            <AppIcon name="info" size="sm" />
          </RuneIconButton>
        </span>
        <span v-if="hint" class="max-w-lg font-rune-reading text-rune-caption text-rune-text-3">{{
          hint
        }}</span>
      </div>
      <div class="flex min-w-0 max-w-full shrink-0 flex-wrap items-center gap-2">
        <slot />
      </div>
    </div>
    <p
      v-if="info && showInfo"
      class="m-0 mt-rune-1 max-w-2xl rounded-rune-tile bg-rune-sunken p-rune-2 text-rune-caption text-rune-text-2"
    >
      {{ info }}
    </p>
  </div>
</template>