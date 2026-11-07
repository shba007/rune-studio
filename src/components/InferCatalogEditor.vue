<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import RuneButton from './RuneButton.vue'
import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'

type Model = Record<string, unknown> & { id: string; name: string }
type Catalog = {
  models: Model[]
  roles?: Record<string, string>
  aliases?: Record<string, string>
  [key: string]: unknown
}
type Document = {
  revision: string
  catalog: string
  connectionId: string
  connectionRevision: number
}
const catalog = ref<Catalog | null>(null)
const revision = ref('')
const connectionId = ref('')
const connectionRevision = ref(0)
const selected = ref(-1)
const editing = ref(false)
const id = ref('')
const name = ref('')
const details = ref('')
const source = ref<'local' | 'remote'>('remote')
const location = ref('')
const provider = ref('')
const upstream = ref('')
const credential = ref('{{PROVIDER_API_KEY}}')
const engine = ref('')
const status = ref('')
const busy = ref(false)
const dirty = ref(false)
const roles = [
  'agent',
  'verifier',
  'analysis',
  'voice',
  'decision',
  'advisor',
  'visual_planner',
  'visual_entities',
  'visual_regions',
  'visual_structure',
  'visual_depth',
  'visual_instances',
  'visual_oriented',
] as const
function select(index: number) {
  editing.value = true
  selected.value = index
  const model = catalog.value?.models[index]
  id.value = model?.id ?? ''
  name.value = model?.name ?? ''
  const local = model?.local as Record<string, unknown> | undefined
  const remote = model?.remote as Record<string, unknown> | undefined
  const engines = model?.engine as { preferred?: string; supported?: string[] } | undefined
  source.value = local ? 'local' : 'remote'
  location.value = String(local?.path ?? '')
  provider.value = String(remote?.provider ?? '')
  upstream.value = String(remote?.upstream_model ?? '')
  credential.value = String(remote?.api_key ?? '{{PROVIDER_API_KEY}}')
  engine.value = engines?.preferred ?? engines?.supported?.[0] ?? ''
  details.value = model
    ? JSON.stringify(model, null, 2)
    : '{\n  "modality": "Text",\n  "capabilities": ["Chat"],\n  "remote": { "provider": "", "upstream_model": "", "api_key": "{{PROVIDER_API_KEY}}" }\n}'
}
async function load() {
  if ((dirty.value || editing.value) && !window.confirm('Discard unsaved catalog changes?')) return
  busy.value = true
  try {
    const result = await invoke<Document>('infer_catalog_read')
    const parsed = JSON.parse(result.catalog) as Catalog
    if (!Array.isArray(parsed.models)) throw new Error('Invalid catalog')
    catalog.value = parsed
    revision.value = result.revision
    connectionId.value = result.connectionId
    connectionRevision.value = result.connectionRevision
    selected.value = -1
    editing.value = false
    dirty.value = false
    status.value = 'Catalog loaded.'
  } catch (error) {
    status.value = String(error)
  } finally {
    busy.value = false
  }
}
function stage() {
  if (!catalog.value) return
  try {
    const model = JSON.parse(details.value) as Model
    if (!model || typeof model !== 'object' || !id.value.trim() || !name.value.trim())
      throw new Error('Enter a model ID and name.')
    model.id = id.value.trim()
    model.name = name.value.trim()
    const previous = catalog.value.models[selected.value]
    if (
      previous &&
      previous.id !== model.id &&
      [
        ...Object.values(catalog.value.roles ?? {}),
        ...Object.values(catalog.value.aliases ?? {}),
      ].includes(previous.id)
    )
      throw new Error('Clear this model from its roles and aliases before changing its ID.')
    if (source.value === 'local') {
      model.local = { ...(model.local as object | undefined), path: location.value }
      const configured = model.engine as
        | { supported?: string[]; [key: string]: unknown }
        | undefined
      const supported = configured?.supported ?? []
      model.engine = {
        ...configured,
        supported: supported.includes(engine.value) ? supported : [...supported, engine.value],
        preferred: engine.value,
      }
      delete model.remote
    } else {
      model.remote = {
        ...(model.remote as object | undefined),
        provider: provider.value,
        upstream_model: upstream.value,
        api_key: credential.value,
      }
      delete model.local
      delete model.engine
    }
    if (
      catalog.value.models.some((item, index) => item.id === model.id && index !== selected.value)
    )
      throw new Error('Model ID already exists.')
    if (selected.value < 0) catalog.value.models.push(model)
    else catalog.value.models[selected.value] = model
    selected.value = -1
    editing.value = false
    dirty.value = true
    status.value = 'Model staged. Save catalog to apply.'
  } catch (error) {
    status.value = String(error)
  }
}
function remove(index: number) {
  const model = catalog.value?.models[index]
  if (
    model &&
    [
      ...Object.values(catalog.value?.roles ?? {}),
      ...Object.values(catalog.value?.aliases ?? {}),
    ].includes(model.id)
  ) {
    status.value = 'Clear this model from its roles and aliases before removing it.'
    return
  }
  catalog.value?.models.splice(index, 1)
  selected.value = -1
  editing.value = false
  dirty.value = true
  status.value = 'Model removed from draft. Save catalog to apply.'
}
function setRole(role: string, event: Event) {
  if (!catalog.value) return
  for (const model of catalog.value.models) {
    if (Array.isArray(model.roles)) {
      model.roles = model.roles.filter(
        (entry) => typeof entry !== 'string' || entry.toLowerCase() !== role.toLowerCase(),
      )
    }
  }
  const roles = { ...catalog.value.roles }
  const value = (event.target as HTMLSelectElement).value
  if (value) roles[role] = value
  else delete roles[role]
  catalog.value.roles = roles
  dirty.value = true
}
function roleValue(role: string): string {
  const assigned = catalog.value?.roles?.[role]
  if (assigned) return assigned
  return (
    catalog.value?.models.find(
      (model) =>
        Array.isArray(model.roles) &&
        model.roles.some(
          (entry) => typeof entry === 'string' && entry.toLowerCase() === role.toLowerCase(),
        ),
    )?.id ?? ''
  )
}
async function save() {
  if (!catalog.value) return
  busy.value = true
  try {
    const result = await invoke<Document>('infer_catalog_write', {
      document: {
        revision: revision.value,
        catalog: JSON.stringify(catalog.value, null, 2) + '\n',
        connectionId: connectionId.value,
        connectionRevision: connectionRevision.value,
      },
    })
    revision.value = result.revision
    dirty.value = false
    status.value = 'Catalog saved. Restart Infer to apply all changes.'
  } catch (error) {
    status.value = String(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <HubGroup title="Infer catalog">
    <div class="flex flex-col gap-rune-2 px-rune-2 text-rune-body">
      <p class="m-0 text-rune-text-2">Manage models and roles on the connected Infer server.</p>
      <p class="m-0 text-rune-text-2">
        Some Studio features still use their own role settings; Infer roles do not yet change every
        model selection.
      </p>
      <div class="flex gap-2">
        <RuneButton size="sm" :disabled="busy" @click="load">Load catalog</RuneButton
        ><RuneButton v-if="catalog" size="sm" :disabled="busy || editing" @click="save"
          >Save catalog</RuneButton
        >
      </div>
      <template v-if="catalog">
        <div
          v-for="(model, index) in catalog.models"
          :key="model.id"
          class="flex items-center gap-2"
        >
          <span class="flex-1">{{ model.name }} ({{ model.id }})</span
          ><RuneButton size="sm" :disabled="editing || busy" @click="select(index)">Edit</RuneButton
          ><RuneButton size="sm" :disabled="editing || busy" @click="remove(index)"
            >Remove</RuneButton
          >
        </div>
        <RuneButton size="sm" :disabled="editing || busy" @click="select(-1)">Add model</RuneButton>
        <template v-if="editing">
          <HubInput v-model="id" label="Model ID" /><HubInput v-model="name" label="Model name" />
          <label
            >Source
            <select v-model="source" class="bg-rune-sunken">
              <option value="local">Local</option>
              <option value="remote">Remote</option>
            </select></label
          >
          <template v-if="source === 'local'"
            ><HubInput v-model="location" label="Model path on Infer server" /><HubInput
              v-model="engine"
              label="Engine"
          /></template>
          <template v-else
            ><HubInput v-model="provider" label="Provider" /><HubInput
              v-model="upstream"
              label="Provider model ID" /><HubInput
              v-model="credential"
              label="API key environment reference"
          /></template>
          <details>
            <summary>Advanced configuration</summary>
            <textarea
              v-model="details"
              aria-label="Advanced model configuration"
              rows="10"
              class="w-full rounded-rune-tile bg-rune-sunken p-rune-2 font-mono"
            />
          </details>
          <RuneButton size="sm" @click="stage">Stage model</RuneButton>
          <RuneButton size="sm" @click="editing = false">Cancel edit</RuneButton>
        </template>
        <label v-for="role in roles" :key="role" class="flex items-center gap-2"
          >{{ role.replaceAll('_', ' ') }}
          <select :value="roleValue(role)" class="bg-rune-sunken" @change="setRole(role, $event)">
            <option value="">Not set</option>
            <option v-for="model in catalog.models" :key="model.id" :value="model.id">
              {{ model.name }}
            </option>
          </select>
        </label>
      </template>
      <p role="status" class="m-0 text-rune-text-2">{{ status }}</p>
    </div>
  </HubGroup>
</template>