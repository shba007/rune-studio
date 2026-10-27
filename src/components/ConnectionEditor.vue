<script setup lang="ts">
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
    <header class="flex items-center justify-between gap-4 border-b border-white/8 pb-5">
      <button
        type="button"
        class="cursor-pointer text-label text-zinc-400 hover:text-white"
        :disabled="busy"
        @click="$emit('cancel')"
      >
        ← Connections
      </button>
      <span class="text-caption text-zinc-500">{{
        connection ? 'Edit connection' : 'New connection'
      }}</span>
    </header>
    <fieldset :disabled="busy" class="m-0 flex flex-col gap-4 border-0 p-0 text-label">
      <label class="flex flex-col gap-2 text-zinc-400"
        >Connection name<input
          v-model="name"
          required
          maxlength="80"
          class="rounded-xl bg-white/6 p-3 text-zinc-100"
      /></label>
      <label class="flex flex-col gap-2 text-zinc-400"
        >Base URL<input
          v-model="baseUrl"
          type="url"
          required
          maxlength="2048"
          placeholder="http://127.0.0.1:8080"
          class="rounded-xl bg-white/6 p-3 text-zinc-100"
      /></label>
      <p class="m-0 text-caption leading-relaxed text-zinc-500">
        Enter the server root without /v1. Studio checks /health and /v1/models.
      </p>
      <label class="flex flex-col gap-2 text-zinc-400"
        >API key
        <span class="text-caption text-zinc-500">{{
          connection
            ? 'Leave blank to keep the current key.'
            : 'Optional. Stored in the system keyring.'
        }}</span
        ><input
          v-model="key"
          type="password"
          autocomplete="off"
          :disabled="clearKey"
          class="rounded-xl bg-white/6 p-3 text-zinc-100"
      /></label>
      <p
        v-if="
          connection?.hasApiKey && baseUrl.trim() !== connection.baseUrl && !clearKey && !key.trim()
        "
        class="m-0 text-caption text-zinc-400"
      >
        Saving will use the current API key with the new address.
      </p>
      <label v-if="connection?.hasApiKey" class="flex items-center gap-2 text-zinc-400"
        ><input v-model="clearKey" type="checkbox" :disabled="!!key.trim()" /> Remove the stored
        key</label
      >
      <p v-if="connection?.id === 'legacy' && clearKey" class="m-0 text-caption text-zinc-500">
        A separately configured RUNE_API_KEY environment value can still apply to this connection.
      </p>
      <div class="flex flex-wrap gap-3 pt-2">
        <button
          type="submit"
          class="cursor-pointer rounded-full bg-zinc-100 px-5 py-2 text-zinc-950 disabled:opacity-40"
        >
          {{ busy ? 'Saving…' : 'Save connection' }}
        </button>
        <button
          type="button"
          class="cursor-pointer rounded-full border border-white/15 px-5 py-2 text-zinc-300"
          @click="$emit('cancel')"
        >
          Cancel
        </button>
      </div>
    </fieldset>
  </form>
</template>