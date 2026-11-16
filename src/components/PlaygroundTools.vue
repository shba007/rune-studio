<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'
import {
  toolArguments,
  toolForm,
  toolResultText,
  type KitTool,
  type ToolField,
} from '@/config/toolForm'
import PlaygroundSettingsPanel from './PlaygroundSettingsPanel.vue'
import RuneButton from './RuneButton.vue'
import RuneNavItem from './RuneNavItem.vue'
import RuneChoice from './RuneChoice.vue'
import AppIcon from './AppIcon.vue'
import ToolResultView from './ToolResultView.vue'
import type { ResultValue } from '@/config/toolResult'

interface Config {
  enabled: boolean
  endpoint: string
  selected: string[]
}
interface Discovery {
  tools: KitTool[]
  unavailable: string[]
}
withDefaults(defineProps<{ sidebarOpen?: boolean }>(), { sidebarOpen: true })
const selectedNames = ref<string[]>([])
const tools = ref<KitTool[]>([])
const unavailable = ref<string[]>([])
const selected = ref('')
const loading = ref(false)
const running = ref(false)
const error = ref('')
const values = ref<Record<string, string>>({})
const raw = ref('{}')
interface ToolCallResult {
  text: string
  structuredContent?: ResultValue
  structuredContentOmitted?: boolean
}
const result = ref<(ToolCallResult & { ok: boolean; ms: number }) | null>(null)
interface Entry {
  id: string
  name: string
  label: string
  tool?: KitTool
  reason?: string
}
const groups = computed(() => {
  const entries: Omit<Entry, 'label'>[] = [
    ...tools.value.map((tool) => ({ id: 'tool:' + tool.name, name: tool.name, tool })),
    ...unavailable.value.map((reason, index) => ({
      id: 'unavailable:' + index,
      name: reason.split(':')[0] || reason,
      reason,
    })),
  ]
  const byNamespace = new Map<string, Entry[]>()
  for (const entry of entries) {
    const separator = entry.name.indexOf('__')
    const namespace = separator < 0 ? entry.name : entry.name.slice(0, separator)
    const group = byNamespace.get(namespace) ?? []
    group.push({ ...entry, label: separator < 0 ? entry.name : entry.name.slice(separator + 2) })
    byNamespace.set(namespace, group)
  }
  return Array.from(byNamespace, ([label, entries]) => ({ label, entries }))
})
const picked = computed(() =>
  groups.value.flatMap((group) => group.entries).find((entry) => entry.id === selected.value),
)
const chosen = computed(() => picked.value?.tool)
const canRun = computed(
  () => !!chosen.value && selectedNames.value.includes(chosen.value.name) && !running.value,
)
const form = computed(() => toolForm(chosen.value?.inputSchema))
watch(selected, () => {
  values.value = {}
  raw.value = '{}'
  result.value = null
})
function options(field: ToolField) {
  return [
    { value: '', label: field.required ? 'Choose' : 'Omit' },
    ...(field.options
      ? field.options.map((value, index) => ({ value: String(index), label: String(value) }))
      : [
          { value: 'true', label: 'True' },
          { value: 'false', label: 'False' },
        ]),
  ]
}
async function discover() {
  if (loading.value || running.value) return
  loading.value = true
  error.value = ''
  tools.value = []
  unavailable.value = []
  selected.value = ''
  result.value = null
  try {
    const config = await invoke<Config>('kit_settings')
    if (!config.enabled) {
      error.value = 'Kit is disabled. Enable Kit tools in Tools settings.'
      return
    }
    const discovery = await invoke<Discovery>('kit_discover', { config })
    selectedNames.value = config.selected
    tools.value = discovery.tools
    unavailable.value = discovery.unavailable ?? []
    selected.value = groups.value.flatMap((group) => group.entries)[0]?.id ?? ''
  } catch (cause) {
    error.value = String(cause)
  } finally {
    loading.value = false
  }
}
async function run() {
  if (!chosen.value || !canRun.value) return
  const name = chosen.value.name
  const started = performance.now()
  running.value = true
  result.value = null
  try {
    const args = toolArguments(form.value, values.value, raw.value)
    const output = await invoke<ToolCallResult>('kit_call_tool', { name, args })
    result.value = {
      ok: true,
      text: toolResultText(output.text),
      structuredContent: output.structuredContent,
      structuredContentOmitted: output.structuredContentOmitted,
      ms: Math.round(performance.now() - started),
    }
  } catch (cause) {
    result.value = { ok: false, text: String(cause), ms: Math.round(performance.now() - started) }
  } finally {
    running.value = false
  }
}
async function openSettings() {
  try {
    await invoke('open_settings_window', { section: 'tools' })
  } catch (cause) {
    error.value = String(cause)
  }
}
onMounted(discover)
</script>

<template>
  <div class="flex min-h-0 flex-1" data-testid="tools-section">
    <nav
      v-show="sidebarOpen"
      class="glass flex w-64 shrink-0 flex-col bg-rune-sunken pb-4"
      aria-label="Tools"
    >
      <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pt-2">
        <section
          v-for="group in groups"
          :key="group.label"
          class="flex flex-col gap-1"
          :aria-label="group.label"
        >
          <h2 class="m-0 px-2 pb-1 text-rune-heading text-rune-text-3 uppercase">
            {{ group.label }}
          </h2>
          <RuneNavItem
            v-for="entry in group.entries"
            :key="entry.id"
            :selected="selected === entry.id"
            :disabled="running"
            :title="entry.name"
            :class="entry.reason ? '!text-rune-text-3' : ''"
            @click="selected = entry.id"
          >
            <AppIcon name="plug" />
            <span class="min-w-0 flex-1 truncate">{{ entry.label }}</span>
            <span
              v-if="entry.tool?.outputSchema !== undefined"
              class="text-rune-caption text-rune-text-3"
              >View</span
            >
            <span
              v-if="entry.reason || !selectedNames.includes(entry.name)"
              class="text-rune-caption text-rune-text-3"
              >{{ entry.reason ? 'Unavailable' : 'Off' }}</span
            >
          </RuneNavItem>
        </section>
      </div>
      <div class="flex shrink-0 flex-wrap gap-1 px-4 pt-4">
        <RuneButton variant="ghost" size="sm" :disabled="loading || running" @click="discover"
          >Refresh</RuneButton
        >
        <RuneButton variant="ghost" size="sm" @click="openSettings">Tools settings</RuneButton>
      </div>
    </nav>
    <PlaygroundSettingsPanel aria-label="Tool settings">
      <header data-tauri-drag-region class="flex h-12 shrink-0 items-center px-6">
        <span class="truncate text-rune-body" :title="picked?.name">{{
          picked?.name ?? 'Tools'
        }}</span>
      </header>
      <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 pb-4">
        <p v-if="loading" role="status" class="m-0 text-rune-body text-rune-text-2">
          Loading tools…
        </p>
        <p v-if="error" role="alert" class="m-0 text-rune-body text-error">{{ error }}</p>
        <p
          v-else-if="!loading && !tools.length && !unavailable.length"
          class="m-0 text-rune-body text-rune-text-2"
        >
          No tools discovered. Check Tools settings.
        </p>
        <p v-if="picked?.reason" class="m-0 text-rune-body text-rune-text-2">{{ picked.reason }}</p>
        <p v-if="chosen" class="m-0 text-rune-body text-rune-text-2">{{ chosen.description }}</p>
        <form v-if="chosen" id="tool-arguments" class="flex flex-col gap-4" @submit.prevent="run">
          <label v-if="form.raw" class="flex flex-col gap-2 text-rune-body">
            Arguments (JSON) *
            <textarea
              v-model="raw"
              aria-label="Arguments (JSON)"
              :disabled="running"
              rows="8"
              class="w-full rounded-rune-tile bg-rune-sunken p-3 text-rune-body"
            />
          </label>
          <template v-else>
            <label
              v-for="field in form.fields"
              :key="field.name"
              class="flex flex-col gap-2 text-rune-body"
            >
              <span>{{ field.label }}{{ field.required ? ' *' : '' }}</span>
              <RuneChoice
                v-if="field.options || field.type === 'boolean'"
                :model-value="values[field.name] ?? ''"
                :label="field.label"
                :options="options(field)"
                :disabled="running"
                menu
                @update:model-value="values[field.name] = $event"
              />
              <input
                v-else
                v-model="values[field.name]"
                :aria-label="field.label"
                :required="field.required"
                :type="field.type === 'string' ? 'text' : 'number'"
                :step="field.type === 'integer' ? '1' : 'any'"
                :disabled="running"
                class="h-10 w-full rounded-rune-tile bg-rune-sunken px-3 text-rune-body"
              />
              <span v-if="field.description" class="text-rune-caption text-rune-text-2">{{
                field.description
              }}</span>
            </label>
          </template>
        </form>
      </div>
      <div v-if="chosen" class="shrink-0 px-6 pb-6">
        <p
          v-if="!selectedNames.includes(chosen.name)"
          class="mb-3 text-rune-caption text-rune-text-2"
        >
          Select this tool in Tools settings to run it
        </p>
        <RuneButton
          type="submit"
          form="tool-arguments"
          variant="primary"
          class="w-full"
          :disabled="!canRun"
          >{{ running ? 'Running…' : 'Run' }}</RuneButton
        >
      </div>
    </PlaygroundSettingsPanel>
    <div class="relative flex min-w-0 flex-1 flex-col">
      <section
        class="m-0 flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-rune-surface bg-[radial-gradient(circle,rgb(255_255_255/0.22)_1px,transparent_1.5px)] bg-size-[24px_24px]"
        aria-label="Tool result"
      >
        <p v-if="running" role="status" class="text-rune-body text-rune-text-2">Running…</p>
        <div v-else-if="result" class="max-h-full w-full max-w-3xl overflow-auto p-6">
          <p role="status" class="text-rune-caption text-rune-text-2">
            {{ result.ok ? 'Succeeded' : 'Failed' }} · {{ result.ms }} ms
          </p>
          <p v-if="result.structuredContentOmitted" class="text-rune-caption text-rune-text-2">
            Structured result omitted for size.
          </p>
          <ToolResultView
            v-if="result.ok"
            :text="result.text"
            :schema="chosen?.outputSchema"
            :structured-content="result.structuredContent"
          />
          <pre
            v-else
            role="alert"
            class="m-0 text-rune-body break-words whitespace-pre-wrap"
            :class="result.ok ? 'text-rune-ink' : 'text-error'"
            >{{ result.text }}</pre>
        </div>
        <div v-else class="flex max-w-80 flex-col items-center gap-3 text-center">
          <AppIcon name="plug" />
          <p class="m-0 text-rune-body text-rune-text-2">
            Results land here. Choose a tool and press Run.
          </p>
        </div>
      </section>
    </div>
  </div>
</template>