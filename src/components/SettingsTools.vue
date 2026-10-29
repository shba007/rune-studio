<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
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
function setEnabled(value: boolean) {
  config.value.enabled = value
  void save()
}
function setSelected(name: string, value: boolean) {
  const others = config.value.selected.filter((item) => item !== name)
  config.value.selected = value ? [...others, name] : others
  void save()
}
</script>
<template>
  <div class="flex flex-col gap-8 text-rune-body">
    <p
      v-if="toolCalls === 'no'"
      role="alert"
      class="m-0 rounded-rune-tile bg-rune-surface p-rune-2 text-attention"
    >
      The selected agent model does not support tool calling. Choose a tool-capable model in Models
      settings.
    </p>
    <HubGroup title="Kit tools">
      <HubToggle
        :model-value="config.enabled"
        label="Enable Kit tools"
        hint="Read-only tools from the shared Kit server."
        info="Start Kit HTTP 0.6.0+ separately with serve --headless. Studio connects to the endpoint and never launches or stops the shared server."
        :disabled="toolCalls === 'no'"
        @update:model-value="setEnabled"
      />
      <HubInput
        v-model="config.endpoint"
        label="Kit HTTP endpoint"
        placeholder="http://127.0.0.1:47601/mcp"
        @commit="save"
      />
      <HubRow label="Discovery" hint="Find the read-only tools this server offers.">
        <RuneButton size="sm" :disabled="busy" @click="refresh">
          {{ busy ? 'Discovering…' : 'Refresh / discover' }}
        </RuneButton>
      </HubRow>
      <template #after>
        <p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ status }}
        </p>
        <p
          v-if="config.migration"
          role="alert"
          class="m-0 px-rune-2 text-rune-caption text-attention"
        >
          {{ config.migration }}
        </p>
        <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          Configure filesystem confinement on the Kit server using an explicit allowed_dir. Studio
          cannot change that root. File contents are sent to the selected model.
        </p>
      </template>
    </HubGroup>
    <HubGroup v-if="tools.length" title="Available tools">
      <HubToggle
        v-for="tool in tools"
        :key="tool.name"
        :model-value="config.selected.includes(tool.name)"
        :label="tool.name"
        :hint="tool.description"
        @update:model-value="setSelected(tool.name, $event)"
      />
    </HubGroup>
  </div>
</template>