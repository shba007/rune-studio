<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'

import {
  MIN_CONTEXT,
  applyRuntime,
  isLocal,
  maxContext,
  readRuntime,
  sameRuntime,
  validateRuntime,
  type Catalog,
  type RuntimeDraft,
} from '@/config/modelRuntime'

import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'
import HubRow from './HubRow.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'

type Document = {
  revision: string
  catalog: string
  connectionId: string
  connectionRevision: number
}

const props = defineProps<{ models?: { id: string; status: string | null }[] }>()

const catalog = ref<Catalog | null>(null)
const document = ref<Omit<Document, 'catalog'> | null>(null)
const selectedId = ref('')
const draft = ref<RuntimeDraft | null>(null)
const baseline = ref<RuntimeDraft | null>(null)
const status = ref('')
const failed = ref(false)
const busy = ref(false)

const model = computed(() => catalog.value?.models.find((item) => item.id === selectedId.value))
const limit = computed(() => (model.value ? maxContext(model.value) : null))
const errors = computed(() => (draft.value ? validateRuntime(draft.value, limit.value) : {}))
const dirty = computed(
  () => !!draft.value && !!baseline.value && !sameRuntime(draft.value, baseline.value),
)
const valid = computed(() => Object.keys(errors.value).length === 0)
const loaded = computed(
  () => props.models?.find((item) => item.id === selectedId.value)?.status === 'loaded',
)
const modelChoices = computed(() =>
  (catalog.value?.models ?? []).map((item) => ({ value: item.id, label: item.name })),
)
const local = computed(() => !!model.value && isLocal(model.value))

function open(id: string) {
  selectedId.value = id
  const entry = catalog.value?.models.find((item) => item.id === id)
  baseline.value = entry ? readRuntime(entry) : null
  draft.value = baseline.value ? { ...baseline.value } : null
}

function take(result: Document, keep?: string) {
  const parsed = JSON.parse(result.catalog) as Catalog
  if (!Array.isArray(parsed.models)) throw new Error('Invalid catalog')
  catalog.value = parsed
  document.value = {
    revision: result.revision,
    connectionId: result.connectionId,
    connectionRevision: result.connectionRevision,
  }
  const first =
    (keep && parsed.models.some((item) => item.id === keep) && keep) ||
    parsed.roles?.agent ||
    parsed.models.find(isLocal)?.id ||
    parsed.models[0]?.id ||
    ''
  open(first)
}

function report(message: string, isError = false) {
  status.value = message
  failed.value = isError
}

async function load() {
  busy.value = true
  try {
    take(await invoke<Document>('infer_catalog_read'), selectedId.value)
    report('')
  } catch (error) {
    report(String(error), true)
  } finally {
    busy.value = false
  }
}

async function write(): Promise<boolean> {
  if (!catalog.value || !document.value || !draft.value || !valid.value) return false
  const next = applyRuntime(catalog.value, selectedId.value, draft.value)
  try {
    const result = await invoke<Document>('infer_catalog_write', {
      document: {
        ...document.value,
        catalog: JSON.stringify(next, null, 2) + '\n',
      },
    })
    take(result, selectedId.value)
    return true
  } catch (error) {
    report(String(error), true)
    return false
  }
}

async function save() {
  busy.value = true
  try {
    if (await write())
      report(
        loaded.value
          ? 'Saved. Press Apply to reload the model with these settings.'
          : 'Saved. The settings apply the next time the model loads.',
      )
  } finally {
    busy.value = false
  }
}

async function apply() {
  busy.value = true
  try {
    if (dirty.value && !(await write())) return
    await invoke('infer_model_residency', { model: selectedId.value, load: false })
    await invoke('infer_model_residency', { model: selectedId.value, load: true })
    report(`Reloaded ${model.value?.name ?? selectedId.value} with the saved settings.`)
  } catch (error) {
    report(String(error), true)
  } finally {
    busy.value = false
  }
}

const EFFORTS = [
  { value: '', label: 'Default' },
  { value: 'none', label: 'None' },
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
]
const CONTEXT_MODES: { value: RuntimeDraft['contextMode']; label: string }[] = [
  { value: 'auto', label: 'Auto' },
  { value: 'manual', label: 'Manual' },
]
const SAMPLING = [
  { field: 'temperature', label: 'Temperature', hint: '0 to 5. Lower is more deterministic.' },
  { field: 'topP', label: 'Top-p', hint: 'Above 0, up to 1.' },
  { field: 'topK', label: 'Top-k', hint: 'A whole number. 0 turns it off.' },
  { field: 'minP', label: 'Min-p', hint: '0 to 1.' },
  { field: 'maxTokens', label: 'Max output tokens', hint: 'The most one reply may use.' },
] as const

const messages = computed(() => Object.values(errors.value))
onMounted(load)
</script>

<template>
  <HubGroup title="Model settings" data-testid="model-runtime">
    <p v-if="!catalog" role="status" class="m-0 px-rune-2 text-rune-body text-rune-text-2">
      {{ failed ? status : 'Loading the model catalog…' }}
    </p>
    <p
      v-else-if="!modelChoices.length"
      role="status"
      class="m-0 px-rune-2 text-rune-body text-rune-text-2"
    >
      No models in the catalog yet.
    </p>
    <template v-else-if="draft">
      <HubRow label="Model" hint="Settings are saved per model in the Infer catalog.">
        <RuneChoice
          :model-value="selectedId"
          label="Model"
          menu
          :options="modelChoices"
          :disabled="busy || dirty"
          @update:model-value="open($event)"
        />
      </HubRow>
      <HubRow
        v-if="local"
        label="Context window"
        :hint="
          draft.contextMode === 'auto'
            ? `Auto uses the model's maximum${limit ? ` (${limit} tokens)` : ''}.`
            : 'The number of tokens the model keeps in memory. More uses more VRAM.'
        "
      >
        <RuneChoice
          v-model="draft.contextMode"
          label="Context window"
          :options="CONTEXT_MODES"
          :disabled="busy"
        />
      </HubRow>
      <HubInput
        v-if="local && draft.contextMode === 'manual'"
        v-model="draft.contextLength"
        label="Context size (tokens)"
        :hint="`${MIN_CONTEXT} to ${limit ?? 'the model maximum'} tokens.`"
        placeholder="e.g. 8192"
      />
      <HubInput
        v-for="row in SAMPLING"
        :key="row.field"
        v-model="draft[row.field]"
        :label="row.label"
        :hint="row.hint"
        placeholder="Default"
      />
      <HubRow label="Reasoning effort" hint="How much the model thinks before it answers.">
        <RuneChoice
          v-model="draft.reasoningEffort"
          label="Reasoning effort"
          menu
          :options="EFFORTS"
          :disabled="busy"
        />
      </HubRow>
      <HubRow
        label="Save changes"
        :hint="
          loaded
            ? 'Apply reloads the model so the new settings take effect now.'
            : 'The model is not loaded; saved settings apply when it loads.'
        "
      >
        <RuneButton size="sm" :disabled="busy || !dirty || !valid" @click="save">Save</RuneButton>
        <RuneButton size="sm" :disabled="busy || !loaded || !valid" @click="apply"
          >Apply</RuneButton
        >
      </HubRow>
    </template>
    <template #after>
      <div
        v-if="messages.length"
        role="alert"
        class="flex flex-col gap-1 px-rune-2 text-rune-body text-attention"
      >
        <p v-for="message in messages" :key="message" class="m-0">{{ message }}</p>
      </div>
      <p
        v-if="status && catalog"
        :role="failed ? 'alert' : 'status'"
        class="m-0 px-rune-2 text-rune-body"
        :class="failed ? 'text-attention' : 'text-rune-text-2'"
      >
        {{ status }}
      </p>
    </template>
  </HubGroup>
</template>