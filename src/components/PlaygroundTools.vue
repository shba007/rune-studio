<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, shallowRef, watch } from 'vue'
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
import StagedChanges from './StagedChanges.vue'
import { wasStaged } from '@/config/stagedChanges'
import type { ResultValue } from '@/config/toolResult'

interface Config {
  enabled: boolean
  endpoint: string
  selected: string[]
}
interface Blocked {
  name: string
  description: string
  kind: 'writes' | 'unverified' | 'unavailable'
  reason: string
  inputSchema?: KitTool['inputSchema']
}
interface Endpoint {
  endpoint: string
}
interface Discovery {
  tools: KitTool[]
  blocked?: Blocked[]
}
withDefaults(defineProps<{ sidebarOpen?: boolean }>(), { sidebarOpen: true })
const selectedNames = ref<string[]>([])
const tools = ref<KitTool[]>([])
const blocked = ref<Blocked[]>([])
// Plugins whose changes Kit stages for review instead of applying at once.
const staged = ref<string[]>([])
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
const result = shallowRef<(ToolCallResult & { ok: boolean; ms: number }) | null>(null)
interface Entry {
  id: string
  name: string
  label: string
  tool?: KitTool
  blocked?: Blocked
}
const BADGES: Record<Blocked['kind'], string> = {
  writes: 'Writes',
  unverified: 'Unverified',
  unavailable: 'Not enabled',
}
const badge = (entry: Entry) =>
  entry.blocked
    ? BADGES[entry.blocked.kind]
    : selectedNames.value.includes(entry.name)
      ? 'Runnable'
      : 'Off'
const groups = computed(() => {
  const entries: Omit<Entry, 'label'>[] = [
    ...tools.value.map((tool) => ({ id: 'tool:' + tool.name, name: tool.name, tool })),
    ...blocked.value.map((item) => ({
      id: 'blocked:' + item.name,
      name: item.name,
      blocked: item,
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
// A write tool is shown with the same form, but only runs after an explicit confirmation.
const writer = computed(() => {
  const blocked = picked.value?.blocked
  return blocked?.kind === 'writes' && blocked.inputSchema
    ? { name: blocked.name, description: blocked.description, inputSchema: blocked.inputSchema }
    : undefined
})
const formTool = computed(() => chosen.value ?? writer.value)
const canRun = computed(
  () => !!chosen.value && selectedNames.value.includes(chosen.value.name) && !running.value,
)
const canReview = computed(() => !!writer.value && !running.value)
const form = computed(() => toolForm(formTool.value?.inputSchema))
const namespaceOf = (name: string) => name.split('__')[0] ?? name
const writerStaged = computed(
  () => !!writer.value && staged.value.includes(namespaceOf(writer.value.name)),
)
const confirming = ref<{ name: string; args: Record<string, unknown> } | null>(null)
const confirmingText = computed(() => JSON.stringify(confirming.value?.args ?? {}, null, 2))
watch(selected, () => {
  confirming.value = null
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
async function stagedPlugins(config: Config & Endpoint, items: Blocked[]) {
  const names = new Set(
    items.filter((item) => item.kind === 'writes').map((i) => namespaceOf(i.name)),
  )
  const staged: string[] = []
  for (const plugin of names) {
    try {
      if (await invoke<boolean>('kit_plugin_staged', { endpoint: config.endpoint, plugin }))
        staged.push(plugin)
    } catch {
      // Not embedded, or unknown: treat as not staged, which shows the stronger warning.
    }
  }
  return staged
}
async function discover() {
  if (loading.value || running.value) return
  loading.value = true
  error.value = ''
  tools.value = []
  blocked.value = []
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
    blocked.value = discovery.blocked ?? []
    staged.value = await stagedPlugins(config, blocked.value)
    selected.value = groups.value.flatMap((group) => group.entries)[0]?.id ?? ''
  } catch (cause) {
    error.value = String(cause)
  } finally {
    loading.value = false
  }
}
function review() {
  if (!writer.value || !canReview.value) return
  try {
    const args = toolArguments(form.value, values.value, raw.value)
    result.value = null
    confirming.value = { name: writer.value.name, args }
  } catch (cause) {
    result.value = { ok: false, text: String(cause), ms: 0 }
  }
}
async function run(confirmed?: { name: string; args: Record<string, unknown> }) {
  if (!confirmed && (!chosen.value || !canRun.value)) return
  const name = confirmed?.name ?? chosen.value!.name
  const started = performance.now()
  running.value = true
  result.value = null
  confirming.value = null
  try {
    const args = confirmed?.args ?? toolArguments(form.value, values.value, raw.value)
    const output = confirmed
      ? await invoke<ToolCallResult>('kit_call_write_tool', { name, args, confirmed: true })
      : await invoke<ToolCallResult>('kit_call_tool', { name, args })
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
            :class="entry.blocked ? '!text-rune-text-3' : ''"
            @click="selected = entry.id"
          >
            <AppIcon name="plug" />
            <span class="min-w-0 flex-1 truncate">{{ entry.label }}</span>
            <span
              v-if="entry.tool?.outputSchema !== undefined"
              class="text-rune-caption text-rune-text-3"
              >View</span
            >
            <span class="text-rune-caption text-rune-text-3">{{ badge(entry) }}</span>
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
          v-else-if="!loading && !tools.length && !blocked.length"
          class="m-0 text-rune-body text-rune-text-2"
        >
          No tools discovered. Check Tools settings.
        </p>
        <p v-if="picked?.blocked" class="m-0 text-rune-body text-rune-text-2">
          {{ picked.blocked.description }}
        </p>
        <p v-if="picked?.blocked" role="status" class="m-0 text-rune-body text-attention">
          {{ BADGES[picked.blocked.kind] }}: {{ picked.blocked.reason }}.
        </p>
        <p v-if="chosen" class="m-0 text-rune-body text-rune-text-2">{{ chosen.description }}</p>
        <form
          v-if="formTool"
          id="tool-arguments"
          class="flex flex-col gap-4"
          @submit.prevent="writer ? review() : run()"
        >
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
      <div v-if="writer" class="shrink-0 px-6 pb-6">
        <div
          v-if="confirming"
          role="alertdialog"
          aria-label="Confirm change"
          class="mb-3 flex flex-col gap-3 rounded-rune-tile bg-rune-sunken p-3"
        >
          <p v-if="writerStaged" class="m-0 text-rune-body text-rune-text-2">
            Kit stages this change. Nothing in the folder changes until you apply it in Tools
            settings, and you can undo it there.
          </p>
          <p v-else class="m-0 text-rune-body text-attention">
            This changes the real folder now. Kit does not stage or undo it.
          </p>
          <p class="m-0 text-rune-caption text-rune-text-2">{{ confirming.name }}</p>
          <pre
            class="m-0 max-h-48 overflow-auto text-rune-caption break-words whitespace-pre-wrap"
            >{{ confirmingText }}</pre>
          <div class="flex gap-2">
            <RuneButton variant="primary" size="sm" @click="run(confirming)"
              >Confirm and run</RuneButton
            >
            <RuneButton variant="ghost" size="sm" @click="confirming = null">Cancel</RuneButton>
          </div>
        </div>
        <RuneButton
          v-else
          type="submit"
          form="tool-arguments"
          variant="primary"
          class="w-full"
          :disabled="!canReview"
          >{{ running ? 'Running…' : 'Review change' }}</RuneButton
        >
      </div>
      <div v-else-if="chosen" class="shrink-0 px-6 pb-6">
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
          <p
            v-if="result.ok && wasStaged(result.structuredContent)"
            role="status"
            class="text-rune-body text-attention"
          >
            Staged, not applied yet.
          </p>
          <StagedChanges
            v-if="result.ok && wasStaged(result.structuredContent) && formTool"
            :key="formTool.name + result.ms"
            class="mb-4"
            :tool="formTool.name"
          />
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