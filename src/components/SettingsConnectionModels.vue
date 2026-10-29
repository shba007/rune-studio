<script setup lang="ts">
import RuneIconButton from './RuneIconButton.vue'
import RuneButton from './RuneButton.vue'
import { computed, ref, watch } from 'vue'
import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'
import BackendBadge from './BackendBadge.vue'
import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
import RuneSwitch from './RuneSwitch.vue'
import ModelRoleRow from './ModelRoleRow.vue'
import AppIcon from './AppIcon.vue'
import ConnectionEditor from './ConnectionEditor.vue'
import {
  useConnections,
  type ConnectionEdit,
  type ConnectionView,
} from '@/composables/useConnections'
const props = defineProps<{ agent: Agent; draft: Settings }>()
const ROLES = [
  { key: 'agent', label: 'Agent' },
  { key: 'verifier', label: 'Verifier' },
  { key: 'analysis', label: 'Visual Analysis' },
  { key: 'voice', label: 'Voice' },
  { key: 'decision', label: 'Decision' },
  { key: 'advisor', label: 'Advisor' },
] as const
const detail = ref(false)
const connections = useConnections()
const editing = ref<ConnectionView | null | undefined>(undefined)
const removing = ref<string | null>(null)
const activeConnection = computed(() =>
  connections.catalog.value?.connections.find((connection) => connection.active),
)
const checking = ref(false)
const modelSearch = ref('')
const selectedDraft = ref<string[]>([])
const additionalDraft = ref('')
const limitDraft = ref('')
const modelError = ref('')
const visibleStatus = computed(() =>
  props.draft.baseUrl === props.agent.settings.value?.baseUrl &&
  (!connections.catalog.value ||
    props.agent.settings.value?.activeConnectionId === connections.catalog.value.activeId) &&
  (!connections.catalog.value ||
    props.agent.settings.value?.connectionsRevision === connections.catalog.value.revision)
    ? props.agent.backend.value
    : null,
)
const discovered = computed(() => visibleStatus.value?.models ?? [])
const additionalIds = computed(() => [
  ...new Set(
    additionalDraft.value
      .split(/[\n,]/)
      .map((id) => id.trim())
      .filter(Boolean),
  ),
])
const modelIds = computed(() => [
  ...new Set([
    ...discovered.value.map((m) => m.id),
    ...additionalIds.value,
    ...selectedDraft.value,
  ]),
])
const filteredModelIds = computed(() =>
  modelIds.value.filter((id) => id.toLowerCase().includes(modelSearch.value.trim().toLowerCase())),
)
const roleModelIds = computed(() => [
  ...new Set([
    ...discovered.value.map((m) => m.id),
    ...(activeConnection.value?.additionalModels ?? []),
  ]),
])
watch(
  activeConnection,
  (connection) => {
    if (!connection) return
    selectedDraft.value =
      connection.selectedModels != null
        ? [...connection.selectedModels]
        : [
            ...new Set([
              ...discovered.value.map((model) => model.id),
              ...Object.values(props.draft.models).filter(Boolean),
            ]),
          ]
    additionalDraft.value = (connection.additionalModels ?? []).join('\n')
    limitDraft.value = connection.outputLimit?.toString() ?? ''
  },
  { immediate: true },
)
function toggleModel(id: string, checked: boolean) {
  selectedDraft.value = checked
    ? [...new Set([...selectedDraft.value, id])]
    : selectedDraft.value.filter((item) => item !== id)
}
async function saveModelSettings() {
  const value = String(limitDraft.value).trim()
  const limit = value ? Number(value) : null
  if (limit !== null && (!Number.isInteger(limit) || limit < 1 || limit > 1000000)) {
    modelError.value = 'Enter an output limit between 1 and 1,000,000 tokens, or leave it blank.'
    return
  }
  if (!activeConnection.value) return
  modelError.value = ''
  await connections.saveModels(
    activeConnection.value.id,
    selectedDraft.value,
    additionalIds.value,
    limit,
  )
}
const filteredModels = computed(() => {
  const query = modelSearch.value.trim().toLowerCase()
  return discovered.value.filter((model) => model.id.toLowerCase().includes(query))
})
const listAvailable = computed(() => {
  const status = visibleStatus.value
  return !!status?.reachable && status.authOk && !status.error && Array.isArray(status.models)
})
async function refresh() {
  if (checking.value) return
  checking.value = true
  try {
    await props.agent.refreshBackend()
  } finally {
    checking.value = false
  }
}
async function saveConnection(edit: ConnectionEdit) {
  if (await connections.save(edit)) editing.value = undefined
}
async function removeConnection(id: string) {
  if (await connections.remove(id)) removing.value = null
}
function openConnection(connection: ConnectionView) {
  if (connection.active) detail.value = true
  else editing.value = connection
}
watch(
  () => connections.catalog.value?.activeId,
  (next, previous) => {
    if (previous && next !== previous) detail.value = false
  },
)
watch(
  () => props.draft.models.agent,
  (value) => {
    props.draft.model = value
  },
)
watch(
  () => props.draft.models.decision,
  (value) => {
    props.draft.decisionModel = value
  },
)
</script>
<template>
  <div class="flex flex-col gap-8">
    <ConnectionEditor
      v-if="editing !== undefined"
      :key="editing?.id ?? 'new'"
      :connection="editing"
      :busy="connections.pending.value"
      @cancel="editing = undefined"
      @save="saveConnection"
    />
    <template v-else-if="!detail">
      <section class="flex flex-col gap-rune-1" aria-label="Model connections">
        <div class="flex flex-wrap items-center justify-between gap-rune-2 px-rune-2">
          <h3 class="m-0 font-sans text-rune-heading text-rune-text-3 uppercase">Connections</h3>
          <RuneButton
            size="sm"
            :disabled="connections.pending.value || !connections.catalog.value"
            @click="editing = null"
          >
            + Add connection
          </RuneButton>
        </div>
        <div class="flex flex-col gap-0.5">
          <div
            v-for="connection in connections.catalog.value?.connections ?? []"
            :key="connection.id"
            class="flex flex-wrap items-center gap-x-4 gap-y-2 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
          >
            <button
              type="button"
              class="min-w-48 flex-1 cursor-pointer text-left"
              :aria-label="`Open ${connection.name}`"
              @click="openConnection(connection)"
            >
              <span class="block text-rune-body text-rune-ink"
                >{{ connection.name }}
                <span v-if="connection.active" class="ml-2 text-rune-caption text-rune-text-3"
                  >Active</span
                ></span
              >
              <span class="mt-1 block break-all text-rune-caption text-rune-text-2">{{
                connection.baseUrl
              }}</span>
              <span
                v-if="connection.active && listAvailable && discovered.length"
                class="mt-1 block truncate text-rune-caption text-rune-text-3"
                >{{
                  discovered
                    .slice(0, 4)
                    .map((model) => model.id)
                    .join(' · ')
                }}{{ discovered.length > 4 ? ' · …' : '' }}</span
              >
            </button>
            <div class="flex flex-wrap items-center gap-rune-1">
              <RuneIconButton
                variant="ghost"
                size="sm"
                :disabled="checking || !!connections.testing.value"
                :label="`Check ${connection.name}`"
                @click="connection.active ? refresh() : connections.check(connection.id)"
              >
                <AppIcon name="refresh" size="sm" />
              </RuneIconButton>
              <RuneButton size="sm" @click="editing = connection">Edit</RuneButton>
              <RuneButton
                v-if="!connection.active"
                variant="primary"
                size="sm"
                :disabled="connections.pending.value || agent.isRunning?.value"
                @click="connections.activate(connection.id)"
              >
                Make active
              </RuneButton>
              <RuneIconButton
                variant="ghost"
                size="sm"
                :disabled="
                  connections.pending.value ||
                  (connections.catalog.value?.connections.length ?? 0) <= 1
                "
                :label="`Remove ${connection.name}`"
                @click="removing = connection.id"
              >
                <AppIcon name="trash" size="sm" />
              </RuneIconButton>
            </div>
            <div class="flex w-full min-w-0 items-center">
              <span
                v-if="
                  (connection.active && checking) || connections.testing.value === connection.id
                "
                role="status"
                class="text-rune-caption text-rune-text-2"
                >Checking…</span
              >
              <BackendBadge v-else-if="connection.active" :status="visibleStatus" />
              <BackendBadge
                v-else-if="connections.statuses.value[connection.id]"
                :status="connections.statuses.value[connection.id]!"
              />
              <span v-else class="text-rune-caption text-rune-text-3">Not checked</span>
            </div>
            <div
              v-if="removing === connection.id"
              class="flex w-full flex-wrap items-center gap-rune-2 rounded-rune-tile bg-rune-sunken p-rune-2 text-rune-caption text-rune-text-2"
            >
              <span class="flex-1"
                >Remove {{ connection.name }} and its stored key?
                {{ connection.active ? 'Another saved connection will become active.' : '' }}</span
              >
              <RuneButton
                variant="danger"
                size="sm"
                :disabled="connections.pending.value"
                @click="removeConnection(connection.id)"
              >
                Confirm remove
              </RuneButton>
              <RuneButton variant="ghost" size="sm" @click="removing = null">Cancel</RuneButton>
            </div>
          </div>
        </div>
        <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          {{
            connections.catalog.value
              ? `${connections.catalog.value.connections.length} ${connections.catalog.value.connections.length === 1 ? 'connection' : 'connections'} · ${listAvailable ? `${discovered.length} ${discovered.length === 1 ? 'model' : 'models'} on active` : 'active model list unavailable'}. One connection is active at a time and model roles are shared.`
              : 'Loading connections…'
          }}
        </p>
      </section>
    </template>
    <template v-else>
      <header class="flex flex-wrap items-center justify-between gap-rune-2">
        <RuneButton variant="link" @click="detail = false">
          <AppIcon name="back" size="sm" /> Connections
        </RuneButton>
        <span class="text-rune-caption text-rune-text-3">Model roles save automatically</span>
      </header>
      <HubGroup title="Connection">
        <HubRow label="Backend">
          <div class="max-w-96 min-w-0">
            <BackendBadge :status="checking ? null : visibleStatus" />
          </div>
          <RuneIconButton
            variant="ghost"
            size="sm"
            label="Check the backend again"
            :disabled="checking"
            @click="refresh"
          >
            <AppIcon name="refresh" size="sm" />
          </RuneIconButton>
        </HubRow>
        <HubRow label="Base URL"
          ><span class="break-all text-rune-body text-rune-text-2">{{
            draft.baseUrl
          }}</span></HubRow
        >
        <HubRow label="API key" hint="Stored separately for this connection.">
          <span class="text-rune-body text-rune-text-2">{{
            activeConnection?.hasApiKey ? 'Configured' : 'Not set'
          }}</span>
          <RuneButton v-if="activeConnection" size="sm" @click="editing = activeConnection">
            Edit connection
          </RuneButton>
        </HubRow>
      </HubGroup>

      <section class="flex flex-col gap-rune-1" aria-label="Discovered models">
        <div class="flex flex-wrap items-center justify-between gap-rune-2 px-rune-2">
          <h3 class="m-0 font-sans text-rune-heading text-rune-text-3 uppercase">
            Discovered models
          </h3>
          <RuneButton variant="ghost" size="sm" :disabled="checking" @click="refresh">
            {{ checking ? 'Checking…' : 'Reload models' }}
          </RuneButton>
        </div>
        <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          {{
            listAvailable
              ? `${discovered.length} ${discovered.length === 1 ? 'model' : 'models'} reported by the server`
              : 'Model list unavailable. Check the connection.'
          }}
        </p>
        <template v-if="listAvailable && discovered.length">
          <input
            v-model="modelSearch"
            type="search"
            aria-label="Search discovered models"
            placeholder="Search models"
            class="h-10 w-full rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
          />
          <ul
            class="m-0 flex max-h-56 list-none flex-col gap-0.5 overflow-y-auto rounded-rune-tile p-0"
          >
            <li
              v-for="model in filteredModels"
              :key="model.id"
              class="bg-rune-surface px-rune-2 py-rune-1 text-rune-body break-all text-rune-text-2"
            >
              {{ model.id }}
            </li>
          </ul>
          <p
            v-if="!filteredModels.length"
            role="status"
            class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
          >
            No matching models.
          </p>
        </template>
        <p v-else-if="listAvailable" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          The server reported no models.
        </p>
      </section>

      <section class="flex flex-col gap-rune-1" aria-label="Model selection and limits">
        <h3 class="m-0 px-rune-2 font-sans text-rune-heading text-rune-text-3 uppercase">
          Model selection and limits
        </h3>
        <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          Select the models available for role assignments. Existing role assignments stay when you
          switch connections; select their models here to assign them again.
        </p>
        <label
          class="flex flex-col gap-2 rounded-rune-tile bg-rune-surface p-rune-2 text-rune-body text-rune-ink"
          >Additional model IDs
          <span class="text-rune-caption text-rune-text-3"
            >One per line or comma separated. Capabilities remain unknown until reported by the
            server.</span
          >
          <textarea
            v-model="additionalDraft"
            rows="3"
            class="rounded-rune-tile bg-rune-sunken p-rune-2 text-rune-body text-rune-ink"
          />
        </label>
        <input
          v-model="modelSearch"
          type="search"
          aria-label="Search selectable models"
          placeholder="Search selectable models"
          class="h-10 w-full rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
        />
        <div class="flex max-h-72 flex-col gap-0.5 overflow-y-auto rounded-rune-tile">
          <div class="flex gap-4 bg-rune-surface px-rune-2 py-rune-1">
            <RuneButton variant="link" @click="selectedDraft = [...modelIds]"
              >Select all</RuneButton
            >
            <RuneButton variant="link" @click="selectedDraft = []">Clear</RuneButton>
          </div>
          <label
            v-for="id in filteredModelIds"
            :key="id"
            class="flex cursor-pointer items-center justify-between gap-rune-2 bg-rune-surface px-rune-2 py-rune-1 text-rune-body text-rune-text-2"
          >
            {{ id }}
            <RuneSwitch
              :model-value="selectedDraft.includes(id)"
              :label="`Select ${id}`"
              @update:model-value="toggleModel(id, $event)"
            />
          </label>
          <p
            v-if="!modelIds.length"
            class="m-0 bg-rune-surface px-rune-2 py-rune-2 text-rune-caption text-rune-text-3"
          >
            No discovered or additional models.
          </p>
        </div>
        <div class="flex flex-col gap-0.5">
          <HubRow
            label="Maximum output tokens"
            hint="Caps model output, not context length."
            info="Requests with lower limits keep their existing limit. Fast checks use a separate API and are not capped. Providers may reject unsupported values."
          >
            <input
              v-model="limitDraft"
              type="number"
              min="1"
              max="1000000"
              aria-label="Maximum output tokens"
              placeholder="No connection cap"
              class="h-10 w-48 rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
            />
          </HubRow>
        </div>
        <p v-if="modelError" role="alert" class="m-0 px-rune-2 text-rune-body text-error">
          {{ modelError }}
        </p>
        <RuneButton
          variant="primary"
          :disabled="connections.pending.value"
          class="mt-rune-1 w-fit"
          @click="saveModelSettings"
        >
          Save model settings
        </RuneButton>
      </section>

      <HubGroup
        title="Model roles"
        hint="Visual Analysis needs an explicit Analysis model with image input and tool calling. Other roles use the agent model when empty."
      >
        <datalist id="discovered-models-list">
          <option v-for="id in roleModelIds" :key="id" :value="id" />
        </datalist>

        <ModelRoleRow
          v-for="role in ROLES"
          :key="role.key"
          v-model="draft.models[role.key]"
          :role-label="role.label"
          :role-key="role.key"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
          :placeholder="role.key === 'agent' ? 'e.g. gemma-4-12b-it' : undefined"
        />
      </HubGroup>

      <HubGroup title="Fast checks">
        <HubToggle
          v-model="draft.fastChecks"
          label="Fast checks"
          hint="Skip planning for confident, simple steps."
          info="A server that loads one model at a time swaps models for every fast check, which can add 10 seconds or more. Leave this off there."
        />
        <HubInput v-if="draft.fastChecks" v-model="draft.decisionUrl" label="Decision URL" />
      </HubGroup>
      <slot />
    </template>
    <p v-if="connections.error.value" role="alert" class="m-0 text-rune-body text-error">
      {{ connections.error.value }}
    </p>
  </div>
</template>