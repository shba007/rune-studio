<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
defineProps<{ toolCalls?: string }>()
interface Config {
  enabled: boolean
  executable: string
  selected: string[]
  root: string
}
interface Tool {
  name: string
  description: string
  inputSchema: unknown
}
const config = ref<Config>({ enabled: false, executable: '', selected: [], root: '' })
const tools = ref<Tool[]>([])
const status = ref('Not connected. rune-kit 0.5.0 or newer with MCP 2026-07-28 is required.')
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
      : `${result.version}: no supported tools. Check installed time/git plugins and filesystem 0.4.1+. File tools require a folder below. Unsafe metadata or unsupported schemas remain unavailable.`
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
  <div class="flex flex-col gap-5 text-body">
    <p v-if="toolCalls === 'no'" role="alert">
      The selected agent model does not support tool calling. Choose a tool-capable model in General
      settings.
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
      >Kit executable (optional)
      <input
        v-model="config.executable"
        class="rounded-lg bg-zinc-900 p-3"
        placeholder="Default: ~/rune/rune-kit.exe (Windows) or ~/rune/rune-kit"
        @change="save"
      />
    </label>
    <p class="m-0 text-zinc-400">
      Read-only tools only. Install rune-kit and plugins separately. Discovery checks ~/rune before
      PATH.
    </p>
    <label class="flex flex-col gap-2"
      >Filesystem folder (optional, absolute path)
      <input
        v-model="config.root"
        class="rounded-lg bg-zinc-900 p-3"
        placeholder="Choose a folder to expose to file tools"
        @change="save"
      />
    </label>
    <p class="m-0 text-zinc-400">
      File contents are sent to the selected model. Choose a trusted folder; these tools do not
      inherit Life protections. Folder changes stop owned gateways. Refresh discovery after changing
      the folder.
    </p>
    <button class="rounded-lg bg-white/10 p-3" :disabled="busy" @click="refresh">
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