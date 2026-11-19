<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'

import { describeChange, pluginOf } from '@/config/stagedChanges'

import RuneButton from './RuneButton.vue'

interface Review {
  path: string
  deleted: boolean
  baseline: string | null
  approval: string
}
interface Recovery {
  path: string
  approval: string
}
interface Inventory {
  plugins: { plugin: string; reviews: Review[]; recoveries: Recovery[] }[]
}
interface Config {
  enabled: boolean
  endpoint: string
  selected: string[]
}

const props = defineProps<{ tool: string }>()
const plugin = computed(() => pluginOf(props.tool))

const config = ref<Config | null>(null)
const reviews = ref<Review[]>([])
const recoveries = ref<Recovery[]>([])
const applied = ref<string[]>([])
const status = ref('')
const failed = ref(false)
const busy = ref(false)
const loaded = ref(false)

const undoable = computed(() =>
  recoveries.value.filter((item) => applied.value.includes(item.path)),
)

async function load() {
  busy.value = true
  try {
    config.value ??= await invoke<Config>('kit_settings')
    const inventory = await invoke<Inventory>('kit_changes_inventory', { config: config.value })
    const mine = inventory.plugins.find((item) => item.plugin === plugin.value)
    reviews.value = mine?.reviews ?? []
    recoveries.value = mine?.recoveries ?? []
    failed.value = false
    status.value = ''
  } catch (error) {
    failed.value = true
    status.value = String(error)
  } finally {
    loaded.value = true
    busy.value = false
  }
}

async function act(action: 'apply' | 'restore', item: Review | Recovery) {
  if (!config.value || busy.value) return
  busy.value = true
  try {
    await invoke('kit_change_action', {
      config: config.value,
      action: { plugin: plugin.value, action, approval: item.approval },
    })
    if (action === 'apply') applied.value = [...applied.value, item.path]
    else applied.value = applied.value.filter((path) => path !== item.path)
    busy.value = false
    await load()
    if (!failed.value) status.value = action === 'apply' ? 'Applied.' : 'Undone.'
  } catch (error) {
    failed.value = true
    status.value = `${String(error)}. Reload and try again.`
    busy.value = false
  }
}

async function applyAll() {
  for (const review of reviews.value) {
    await act('apply', review)
    if (failed.value) return
  }
}

onMounted(load)
</script>

<template>
  <section
    class="flex min-w-0 flex-col gap-2 rounded-rune-tile bg-rune-sunken p-3"
    aria-label="Staged changes"
  >
    <p v-if="!loaded" role="status" class="m-0 text-rune-caption text-rune-text-2">
      Checking for staged changes…
    </p>
    <template v-else>
      <div v-for="review in reviews" :key="review.approval" class="flex flex-col gap-1">
        <p class="m-0 text-rune-body break-all">{{ review.path }}</p>
        <p class="m-0 text-rune-caption text-rune-text-2">
          {{ describeChange(review) }}. Not applied yet.
        </p>
        <div>
          <RuneButton size="sm" variant="primary" :disabled="busy" @click="act('apply', review)"
            >Apply</RuneButton
          >
        </div>
      </div>
      <RuneButton
        v-if="reviews.length > 1"
        size="sm"
        :disabled="busy"
        data-testid="apply-all"
        @click="applyAll"
        >Apply all</RuneButton
      >
      <div v-for="item in undoable" :key="item.approval" class="flex items-center gap-2">
        <span class="min-w-0 flex-1 text-rune-caption break-all text-rune-text-2"
          >{{ item.path }} applied</span
        >
        <RuneButton size="sm" :disabled="busy" @click="act('restore', item)">Undo</RuneButton>
      </div>
      <p
        v-if="!reviews.length && !undoable.length && !failed"
        role="status"
        class="m-0 text-rune-caption text-rune-text-2"
      >
        No staged changes waiting.
      </p>
      <p
        v-if="status"
        :role="failed ? 'alert' : 'status'"
        class="m-0 text-rune-caption"
        :class="failed ? 'text-attention' : 'text-rune-text-2'"
      >
        {{ status }}
      </p>
      <RuneButton v-if="failed" size="sm" :disabled="busy" @click="load">Reload</RuneButton>
    </template>
  </section>
</template>