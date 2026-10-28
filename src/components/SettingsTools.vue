<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
defineProps<{ toolCalls?: string }>()
interface Config {
  enabled: boolean
  endpoint: string
  migration?: string
  selected: string[]
}
interface Tool {
  name: string
  description: string
  inputSchema: unknown
}
const config = ref<Config>({ enabled: false, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] })
const tools = ref<Tool[]>([])
const status = ref(
  'Not connected. rune-kit 0.6.0 or newer with a compatible MCP protocol is required.',
)
const busy = ref(false)
async function save() {
  try {
    await invoke('kit_save', { config: config.value })
  } catch (error) {
    status.value = String(error)
  }
}
async function refresh() {
  busy.value = true
  try {
    const result = await invoke<{ version: string; tools: Tool[]; unavailable?: string[] }>(
      'kit_discover',
      {
        config: config.value,
      },
    )
    tools.value = result.tools
    status.value = result.tools.length
      ? `${result.version}: ${result.tools.length} supported read-only tools available.`
      : `${result.version}: no supported tools. Check installed time/git plugins and filesystem 0.4.1+. File tools require a server-configured allowed_dir. Unsafe metadata or unsupported schemas remain unavailable.`
    if (result.unavailable?.length) status.value += ` ${result.unavailable.join('; ')}`
  } catch (error) {
    tools.value = []
    status.value = String(error)
  } finally {
    busy.value = false
  }
}
onMounted(async () => {
  try {
    config.value = await invoke<Config>('kit_settings')
  } catch (error) {
    status.value = String(error)
  }
})
</script>
<template>
  <div class="flex flex-col gap-rune-3 text-rune-body">
    <p v-if="toolCalls === 'no'" role="alert">
      The selected agent model does not support tool calling. Choose a tool-capable model in
      Connections settings.
    </p>
    <label
      ><input
        v-model="config.enabled"
        type="checkbox"
        :disabled="toolCalls === 'no'"
        @change="save"
      />
      Enable Kit tools</label
    >
    <label class="flex flex-col gap-2"
      >Kit HTTP endpoint
      <input
        v-model="config.endpoint"
        class="rounded-lg bg-zinc-900 p-rune-2"
        placeholder="http://127.0.0.1:47601/mcp"
        @change="save"
      />
    </label>
    <p class="m-0 text-zinc-400">
      Read-only tools only. Start Kit HTTP 0.6.0+ separately with serve --headless. Studio connects
      to the endpoint and never launches or stops the shared server.
    </p>
    <p v-if="config.migration" role="alert">{{ config.migration }}</p>
    <p class="m-0 text-zinc-400">
      Configure filesystem confinement on the Kit server using an explicit allowed_dir. Studio
      cannot change that root. File contents are sent to the selected model.
    </p>
    <button class="rounded-lg bg-white/10 p-rune-2" :disabled="busy" @click="refresh">
      {{ busy ? 'Discovering…' : 'Refresh / discover' }}
    </button>
    <p role="status">{{ status }}</p>
    <label v-for="tool in tools" :key="tool.name"
      ><input v-model="config.selected" type="checkbox" :value="tool.name" @change="save" />
      {{ tool.name }}<span class="block text-zinc-500">{{ tool.description }}</span></label
    >
    <p v-if="config.selected.length" class="text-zinc-400">
      Selected: {{ config.selected.join(', ') }}. Selection never grants tools permission to change
      these settings.
    </p>
  </div>
</template>