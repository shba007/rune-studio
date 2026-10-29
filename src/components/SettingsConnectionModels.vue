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
import ModelRoleRow from './ModelRoleRow.vue'
import AppIcon from './AppIcon.vue'
import ConnectionEditor from './ConnectionEditor.vue'
import {
  useConnections,
  type ConnectionEdit,
  type ConnectionView,
} from '@/composables/useConnections'
const props = defineProps<{ agent: Agent; draft: Settings }>()
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
      <div class="overflow-hidden rounded-rune-tile bg-rune-sunken" aria-label="Model connections">
        <div class="flex flex-wrap items-center justify-between gap-rune-2 px-rune-2 py-4">
          <RuneButton
            :disabled="connections.pending.value || !connections.catalog.value"
            @click="editing = null"
          >
            + Add connection
          </RuneButton>
          <span class="text-rune-caption text-rune-text-2">{{
            connections.catalog.value
              ? `${connections.catalog.value.connections.length} ${connections.catalog.value.connections.length === 1 ? 'connection' : 'connections'} · ${listAvailable ? `${discovered.length} ${discovered.length === 1 ? 'model' : 'models'} on active` : 'active model list unavailable'}`
              : 'Loading connections…'
          }}</span>
        </div>
        <div
          v-for="connection in connections.catalog.value?.connections ?? []"
          :key="connection.id"
          class="flex flex-wrap items-center gap-4 px-rune-2 py-rune-3"
        >
          <span class="grid size-12 shrink-0 place-items-center rounded-rune-tile bg-black"
            ><AppIcon name="logo" class="text-rune-ink"
          /></span>
          <button
            type="button"
            class="min-w-0 flex-1 cursor-pointer text-left"
            :aria-label="`Open ${connection.name}`"
            @click="openConnection(connection)"
          >
            <span class="block text-rune-body font-medium text-rune-ink"
              >{{ connection.name }}
              <span v-if="connection.active" class="ml-2 text-rune-caption text-rune-text-2"
                >Active</span
              ></span
            >
            <span class="mt-1 block break-all text-rune-body text-rune-text-2">{{
              connection.baseUrl
            }}</span>
            <span
              v-if="connection.active && listAvailable && discovered.length"
              class="mt-2 block truncate text-rune-caption text-rune-text-2"
              >{{
                discovered
                  .slice(0, 4)
                  .map((model) => model.id)
                  .join(' · ')
              }}{{ discovered.length > 4 ? ' · …' : '' }}</span
            >
          </button>
          <div class="flex flex-wrap items-center gap-rune-2">
            <span
              v-if="(connection.active && checking) || connections.testing.value === connection.id"
              role="status"
              class="text-rune-body text-rune-text-2"
              >Checking…</span
            >
            <BackendBadge v-else-if="connection.active" :status="visibleStatus" />
            <BackendBadge
              v-else-if="connections.statuses.value[connection.id]"
              :status="connections.statuses.value[connection.id]!"
            />
            <span v-else class="text-rune-caption text-rune-text-2">Not checked</span>
            <RuneIconButton
              variant="ghost"
              size="sm"
              :disabled="checking || !!connections.testing.value"
              :label="`Check ${connection.name}`"
              @click="connection.active ? refresh() : connections.check(connection.id)"
            >
              <AppIcon name="refresh" />
            </RuneIconButton>
            <RuneButton @click="editing = connection"> Edit </RuneButton>
            <RuneButton
              variant="primary"
              v-if="!connection.active"
              :disabled="connections.pending.value || agent.isRunning?.value"
              @click="connections.activate(connection.id)"
            >
              Make active
            </RuneButton>
            <RuneButton
              variant="ghost"
              size="sm"
              :disabled="
                connections.pending.value ||
                (connections.catalog.value?.connections.length ?? 0) <= 1
              "
              :aria-label="`Remove ${connection.name}`"
              @click="removing = connection.id"
            >
              Remove
            </RuneButton>
          </div>
          <div
            v-if="removing === connection.id"
            class="flex w-full flex-wrap items-center gap-rune-2 pt-4 text-rune-body text-rune-text-2"
          >
            <span class="flex-1"
              >Remove {{ connection.name }} and its stored key?
              {{ connection.active ? 'Another saved connection will become active.' : '' }}</span
            >
            <RuneButton
              variant="ghost"
              size="sm"
              :disabled="connections.pending.value"
              @click="removeConnection(connection.id)"
            >
              Confirm remove
            </RuneButton>
            <RuneButton variant="ghost" size="sm" @click="removing = null"> Cancel </RuneButton>
          </div>
        </div>
      </div>
      <p class="m-0 text-rune-body leading-relaxed text-rune-text-2">
        One connection is active at a time. Model roles are shared. Kit tools are configured under
        Kit &amp; tools.
      </p>
    </template>
    <template v-else>
      <header class="flex flex-wrap items-center justify-between gap-rune-2 pb-rune-3">
        <RuneButton variant="link" @click="detail = false">
          <AppIcon name="back" size="sm" /> Connections
        </RuneButton>
        <span class="text-rune-caption text-rune-text-2">Model roles save automatically</span>
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
            <AppIcon name="refresh" />
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
          <RuneButton v-if="activeConnection" @click="editing = activeConnection">
            Edit connection
          </RuneButton>
        </HubRow>
      </HubGroup>

      <section class="flex flex-col gap-rune-2 pt-6" aria-label="Discovered models">
        <div class="flex flex-wrap items-center justify-between gap-rune-2">
          <div>
            <h2 class="m-0 text-rune-body text-rune-ink">Discovered models</h2>
            <p class="mt-1 mb-0 text-rune-caption text-rune-text-2">
              {{
                listAvailable
                  ? `${discovered.length} ${discovered.length === 1 ? 'model' : 'models'} reported by the server`
                  : 'Model list unavailable. Check the connection.'
              }}
            </p>
          </div>
          <RuneButton variant="ghost" size="sm" :disabled="checking" @click="refresh">
            {{ checking ? 'Checking…' : 'Reload models' }}
          </RuneButton>
        </div>
        <template v-if="listAvailable && discovered.length">
          <input
            v-model="modelSearch"
            type="search"
            aria-label="Search discovered models"
            placeholder="Search models"
            class="h-10 w-full rounded-rune-tile bg-rune-surface px-rune-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
          />
          <ul class="m-0 max-h-56 list-none overflow-y-auto rounded-rune-tile p-0 bg-rune-raised">
            <li
              v-for="model in filteredModels"
              :key="model.id"
              class="break-all px-4 py-rune-2 text-rune-body text-rune-text-2"
            >
              {{ model.id }}
            </li>
          </ul>
          <p
            v-if="!filteredModels.length"
            role="status"
            class="m-0 text-rune-caption text-rune-text-2"
          >
            No matching models.
          </p>
        </template>
        <p v-else-if="listAvailable" class="m-0 text-rune-caption text-rune-text-2">
          The server reported no models.
        </p>
      </section>

      <section class="flex flex-col gap-4 pt-6" aria-label="Model selection and limits">
        <h2 class="m-0 text-rune-body text-rune-ink">Model selection and limits</h2>
        <p class="m-0 text-rune-body text-rune-text-2">
          Select the models available for role assignments.
        </p>
        <p class="m-0 text-rune-caption text-rune-text-2">
          Existing role assignments remain unchanged when you switch connections. Select their
          models here to assign them again.
        </p>
        <label class="flex flex-col gap-2 text-rune-body text-rune-text-2"
          >Additional model IDs
          <span class="text-rune-caption text-rune-text-2"
            >One per line or comma separated. Capabilities remain unknown until reported by the
            server.</span
          >
          <textarea
            v-model="additionalDraft"
            rows="3"
            class="rounded-rune-tile bg-rune-surface p-rune-2 text-rune-ink"
          />
        </label>
        <input
          v-model="modelSearch"
          type="search"
          aria-label="Search selectable models"
          placeholder="Search selectable models"
          class="h-10 w-full rounded-rune-tile bg-rune-surface px-rune-2 text-rune-body text-rune-ink"
        />
        <div class="max-h-56 overflow-y-auto rounded-rune-tile bg-rune-raised">
          <div class="flex gap-4 px-4 py-rune-2 text-rune-body text-rune-text-2">
            <RuneButton variant="link" @click="selectedDraft = [...modelIds]"
              >Select all</RuneButton
            >
            <RuneButton variant="link" @click="selectedDraft = []">Clear</RuneButton>
          </div>
          <label
            v-for="id in filteredModelIds"
            :key="id"
            class="flex items-center gap-rune-2 px-4 py-rune-2 text-rune-body text-rune-text-2"
          >
            <input
              type="checkbox"
              :checked="selectedDraft.includes(id)"
              @change="toggleModel(id, ($event.target as HTMLInputElement).checked)"
            />{{ id }}
          </label>
          <p v-if="!modelIds.length" class="px-4 text-rune-caption text-rune-text-2">
            No discovered or additional models.
          </p>
        </div>
        <label class="flex flex-col gap-2 text-rune-body text-rune-text-2"
          >Maximum output tokens
          <input
            v-model="limitDraft"
            type="number"
            min="1"
            max="1000000"
            placeholder="No connection cap"
            class="rounded-rune-tile bg-rune-surface p-rune-2 text-rune-ink"
          />
        </label>
        <p class="m-0 text-rune-caption text-rune-text-2">
          Caps model output, not context length. Requests with lower limits keep their existing
          limit. Fast checks use a separate API and are not capped. Providers may reject unsupported
          values.
        </p>
        <p v-if="modelError" role="alert" class="m-0 text-rune-body text-error">
          {{ modelError }}
        </p>
        <RuneButton
          variant="primary"
          :disabled="connections.pending.value"
          class="w-fit"
          @click="saveModelSettings"
        >
          Save model settings
        </RuneButton>
      </section>

      <HubGroup title="Models">
        <p class="m-0 mb-2 text-rune-body text-rune-text-2">
          Choose a model for each job. Visual Analysis requires an explicit Analysis model with
          image input and tool calling. Other optional roles can use the agent model when empty.
          Open Model capabilities to check what each model supports.
        </p>
        <datalist id="discovered-models-list">
          <option v-for="id in roleModelIds" :key="id" :value="id" />
        </datalist>

        <ModelRoleRow
          v-model="draft.models.agent"
          role-label="Agent"
          role-key="agent"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
          placeholder="e.g. gemma-4-12b-it"
        />
        <ModelRoleRow
          v-model="draft.models.verifier"
          role-label="Verifier"
          role-key="verifier"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
        />
        <ModelRoleRow
          v-model="draft.models.analysis"
          role-label="Visual Analysis"
          role-key="analysis"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
        />
        <ModelRoleRow
          v-model="draft.models.voice"
          role-label="Voice"
          role-key="voice"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
        />
        <ModelRoleRow
          v-model="draft.models.decision"
          role-label="Decision"
          role-key="decision"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
        />
        <ModelRoleRow
          v-model="draft.models.advisor"
          role-label="Advisor"
          role-key="advisor"
          :agent-model="draft.models.agent"
          :models="discovered"
          :selected-models="activeConnection?.selectedModels"
        />

        <HubToggle
          v-model="draft.fastChecks"
          label="Fast checks"
          hint="A server that loads one model at a time swaps models for every fast check, which can add 10 seconds or more. Leave this off there."
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