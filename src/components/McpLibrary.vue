<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { isEnabled, toolsOf, withEnabled } from '@/config/mcpTools'
import McpIcon from './McpIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneSwitch from './RuneSwitch.vue'

type Config = { enabled: boolean; endpoint: string; selected: string[] }
type Installed = { name: string; version: string; description?: string | null }
type Entry = { name: string; description: string; latest: string; has_icon: boolean }

const config = defineModel<Config>('config', { required: true })
const emit = defineEmits<{ changed: [] }>()

const tab = ref<'discover' | 'installed'>('installed')
const search = ref('')
const installed = ref<Installed[]>([])
const registry = ref<Entry[]>([])
const toolNames = ref<string[]>([])
const loading = ref(true)
const busy = ref('')
const removing = ref<string | null>(null)
const error = ref('')
const note = ref('')

const query = computed(() => search.value.trim().toLowerCase())
const matches = (name: string, description?: string | null) =>
  `${name} ${description ?? ''}`.toLowerCase().includes(query.value)
const installedNames = computed(() => new Set(installed.value.map((item) => item.name)))
const shownInstalled = computed(() =>
  installed.value.filter((item) => matches(item.name, item.description)),
)
const shownRegistry = computed(() =>
  registry.value.filter((item) => matches(item.name, item.description)),
)

async function apply(next: Config) {
  config.value = next
  await invoke('kit_save', { config: next })
}
async function loadInstalled() {
  const result = await invoke<{ mcps?: Installed[]; plugins?: Installed[] }>('kit_packages', {
    config: config.value,
  })
  installed.value = result?.mcps ?? result?.plugins ?? []
}
async function loadTools() {
  try {
    const result = await invoke<{ tools: Array<{ name: string }> }>('kit_discover', {
      config: config.value,
    })
    toolNames.value = (result?.tools ?? []).map((tool) => tool.name)
  } catch {
    toolNames.value = []
  }
}
async function load() {
  error.value = ''
  try {
    await loadInstalled()
  } catch (e) {
    installed.value = []
    error.value = String(e)
  }
  try {
    const list = await invoke<Entry[]>('kit_registry', { config: config.value })
    registry.value = Array.isArray(list) ? list : []
  } catch {
    registry.value = []
  }
  await loadTools()
  loading.value = false
}
async function toggle(item: Installed, enabled: boolean) {
  error.value = ''
  try {
    await apply({
      ...config.value,
      enabled: enabled ? true : config.value.enabled,
      selected: withEnabled(item.name, toolNames.value, config.value.selected, enabled),
    })
    emit('changed')
  } catch (e) {
    error.value = String(e)
  }
}
async function install(entry: Entry) {
  busy.value = entry.name
  error.value = ''
  note.value = ''
  try {
    const result = await invoke<{ version: string; restart_required?: boolean }>(
      'kit_install_package',
      {
        config: config.value,
        package: { kind: 'mcp', target: entry.name, version: null, prefer_native: false },
      },
    )
    await loadInstalled()
    await loadTools()
    if (toolsOf(entry.name, toolNames.value).length) {
      await apply({
        ...config.value,
        enabled: true,
        selected: withEnabled(entry.name, toolNames.value, config.value.selected, true),
      })
      note.value = `${entry.name} ${result.version} installed and enabled.`
    } else {
      note.value = `${entry.name} ${result.version} installed.${result.restart_required ? ' Restart Kit to load it.' : ''}`
    }
    tab.value = 'installed'
    emit('changed')
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = ''
  }
}
async function remove(item: Installed) {
  busy.value = item.name
  error.value = ''
  try {
    await invoke('kit_package_operation', {
      config: config.value,
      action: 'remove',
      kind: 'mcp',
      name: item.name,
    })
    await apply({
      ...config.value,
      selected: withEnabled(item.name, toolNames.value, config.value.selected, false),
    })
    removing.value = null
    await loadInstalled()
    await loadTools()
    emit('changed')
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = ''
  }
}
onMounted(load)
</script>

<template>
  <section class="flex flex-col gap-rune-1" aria-label="MCP servers">
    <div class="flex flex-wrap items-center justify-between gap-rune-2 px-rune-2">
      <div role="tablist" aria-label="MCP library" class="flex gap-rune-1">
        <RuneButton
          role="tab"
          size="sm"
          :variant="tab === 'installed' ? 'primary' : 'ghost'"
          :aria-selected="tab === 'installed'"
          @click="tab = 'installed'"
          >Installed</RuneButton
        >
        <RuneButton
          role="tab"
          size="sm"
          :variant="tab === 'discover' ? 'primary' : 'ghost'"
          :aria-selected="tab === 'discover'"
          @click="tab = 'discover'"
          >Discover</RuneButton
        >
      </div>
    </div>
    <input
      v-model="search"
      type="search"
      aria-label="Search MCP servers"
      placeholder="Search MCP servers"
      class="h-10 w-full rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
    />
    <p v-if="loading" role="status" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
      Loading MCP servers…
    </p>

    <template v-else-if="tab === 'installed'">
      <p
        v-if="!installed.length && !error"
        role="status"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
      >
        No MCP servers are installed. Open Discover to add one.
      </p>
      <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
        <li
          v-for="item in shownInstalled"
          :key="item.name"
          class="flex flex-wrap items-center gap-rune-2 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
        >
          <McpIcon :name="item.name" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-rune-body text-rune-ink">{{ item.name }}</span>
            <span class="block truncate text-rune-caption text-rune-text-3">
              {{ item.version }}{{ item.description ? ` · ${item.description}` : '' }}
            </span>
          </span>
          <RuneButton
            variant="ghost"
            size="sm"
            :disabled="!!busy"
            :aria-label="`Remove ${item.name}`"
            @click="removing = item.name"
            >Remove</RuneButton
          >
          <RuneSwitch
            :model-value="isEnabled(item.name, toolNames, config.selected)"
            :label="`Enable ${item.name}`"
            :disabled="!!busy || !toolsOf(item.name, toolNames).length"
            @update:model-value="toggle(item, $event)"
          />
          <div
            v-if="removing === item.name"
            class="flex w-full flex-wrap items-center gap-rune-2 rounded-rune-tile bg-rune-sunken p-rune-2 text-rune-caption text-rune-text-2"
          >
            <span class="flex-1">Remove {{ item.name }} and its files?</span>
            <RuneButton variant="danger" size="sm" :disabled="!!busy" @click="remove(item)"
              >Confirm remove</RuneButton
            >
            <RuneButton variant="ghost" size="sm" @click="removing = null">Cancel</RuneButton>
          </div>
        </li>
      </ul>
    </template>

    <template v-else>
      <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
        Reviewed MCP servers from the Rune tools registry.
      </p>
      <p
        v-if="!registry.length"
        role="status"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
      >
        The registry is unavailable. Check that Kit is running and online.
      </p>
      <ul class="m-0 flex list-none flex-col gap-0.5 p-0">
        <li
          v-for="entry in shownRegistry"
          :key="entry.name"
          class="flex items-center gap-rune-2 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
        >
          <McpIcon :name="entry.name" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-rune-body text-rune-ink">{{ entry.name }}</span>
            <span class="block truncate text-rune-caption text-rune-text-3">
              {{ entry.latest }}{{ entry.description ? ` · ${entry.description}` : '' }}
            </span>
          </span>
          <span v-if="installedNames.has(entry.name)" class="text-rune-caption text-rune-text-3"
            >Installed</span
          >
          <RuneButton
            v-else
            size="sm"
            :disabled="!!busy"
            :aria-label="`Install ${entry.name}`"
            @click="install(entry)"
            >{{ busy === entry.name ? 'Installing…' : 'Install' }}</RuneButton
          >
        </li>
      </ul>
    </template>
    <p v-if="note" role="status" class="m-0 px-rune-2 text-rune-caption text-rune-text-2">
      {{ note }}
    </p>
    <p v-if="error" role="alert" class="m-0 px-rune-2 text-rune-body text-attention">
      {{ error }}
    </p>
  </section>
</template>