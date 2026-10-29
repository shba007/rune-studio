<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { ref } from 'vue'
import type { ConnectionEdit, ConnectionView } from '@/composables/useConnections'
const props = defineProps<{ connection: ConnectionView | null; busy: boolean }>()
const emit = defineEmits<{ cancel: []; save: [edit: ConnectionEdit] }>()
const name = ref(props.connection?.name ?? '')
const baseUrl = ref(props.connection?.baseUrl ?? '')
const key = ref('')
const clearKey = ref(false)
function submit() {
  emit('save', {
    id: props.connection?.id ?? null,
    name: name.value,
    baseUrl: baseUrl.value,
    key: key.value,
    clearKey: clearKey.value,
  })
}
</script>
<template>
  <form aria-label="Edit connection" class="flex flex-col gap-5" @submit.prevent="submit">
    <header class="flex items-center justify-between gap-4 pb-5">
      <RuneButton variant="ghost" size="sm" :disabled="busy" @click="$emit('cancel')">
        ← Connections
      </RuneButton>
      <span class="text-rune-heading text-rune-text-2">{{
        connection ? 'Edit connection' : 'New connection'
      }}</span>
    </header>
    <fieldset :disabled="busy" class="m-0 flex flex-col gap-4 p-0 text-rune-caption">
      <label class="flex flex-col gap-2 text-rune-text-2"
        >Connection name<input
          v-model="name"
          required
          maxlength="80"
          class="rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
      /></label>
      <label class="flex flex-col gap-2 text-rune-text-2"
        >Base URL<input
          v-model="baseUrl"
          type="url"
          required
          maxlength="2048"
          placeholder="http://127.0.0.1:8080"
          class="rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
      /></label>
      <p class="m-0 text-rune-heading leading-relaxed text-rune-text-2">
        Enter the server root without /v1. Studio checks /health and /v1/models.
      </p>
      <label class="flex flex-col gap-2 text-rune-text-2"
        >API key
        <span class="text-rune-heading text-rune-text-2">{{
          connection
            ? 'Leave blank to keep the current key.'
            : 'Optional. Stored in the system keyring.'
        }}</span
        ><input
          v-model="key"
          type="password"
          autocomplete="off"
          :disabled="clearKey"
          class="rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
      /></label>
      <p
        v-if="
          connection?.hasApiKey && baseUrl.trim() !== connection.baseUrl && !clearKey && !key.trim()
        "
        class="m-0 text-rune-heading text-rune-text-2"
      >
        Saving will use the current API key with the new address.
      </p>
      <label v-if="connection?.hasApiKey" class="flex items-center gap-2 text-rune-text-2"
        ><input v-model="clearKey" type="checkbox" :disabled="!!key.trim()" /> Remove the stored
        key</label
      >
      <p
        v-if="connection?.id === 'legacy' && clearKey"
        class="m-0 text-rune-heading text-rune-text-2"
      >
        A separately configured RUNE_API_KEY environment value can still apply to this connection.
      </p>
      <div class="flex flex-wrap gap-3 pt-2">
        <RuneButton variant="primary" type="submit">
          {{ busy ? 'Saving…' : 'Save connection' }}
        </RuneButton>
        <RuneButton @click="$emit('cancel')"> Cancel </RuneButton>
      </div>
    </fieldset>
  </form>
</template>