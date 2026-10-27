<script setup lang="ts">
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
      <div
        class="overflow-hidden rounded-2xl border border-white/10 bg-[#101010]"
        aria-label="Model connections"
      >
        <div
          class="flex flex-wrap items-center justify-between gap-3 border-b border-white/8 px-5 py-4"
        >
          <button
            type="button"
            :disabled="connections.pending.value || !connections.catalog.value"
            class="cursor-pointer rounded-full border border-white/15 px-4 py-2 text-label text-zinc-200 disabled:opacity-40"
            @click="editing = null"
          >
            + Add connection
          </button>
          <span class="text-caption text-zinc-500">{{
            connections.catalog.value
              ? `${connections.catalog.value.connections.length} ${connections.catalog.value.connections.length === 1 ? 'connection' : 'connections'} · ${listAvailable ? `${discovered.length} ${discovered.length === 1 ? 'model' : 'models'} on active` : 'active model list unavailable'}`
              : 'Loading connections…'
          }}</span>
        </div>
        <div
          v-for="connection in connections.catalog.value?.connections ?? []"
          :key="connection.id"
          class="flex flex-wrap items-center gap-4 border-b border-white/8 px-5 py-5 last:border-0"
        >
          <span
            class="grid size-12 shrink-0 place-items-center rounded-xl border border-white/10 bg-black"
            ><AppIcon name="logo" class="text-zinc-200"
          /></span>
          <button
            type="button"
            class="min-w-0 flex-1 cursor-pointer text-left"
            :aria-label="`Open ${connection.name}`"
            @click="openConnection(connection)"
          >
            <span class="block text-body font-medium text-zinc-100"
              >{{ connection.name }}
              <span v-if="connection.active" class="ml-2 text-caption text-zinc-400"
                >Active</span
              ></span
            >
            <span class="mt-1 block break-all text-label text-zinc-400">{{
              connection.baseUrl
            }}</span>
            <span
              v-if="connection.active && listAvailable && discovered.length"
              class="mt-2 block truncate text-caption text-zinc-500"
              >{{
                discovered
                  .slice(0, 4)
                  .map((model) => model.id)
                  .join(' · ')
              }}{{ discovered.length > 4 ? ' · …' : '' }}</span
            >
          </button>
          <div class="flex flex-wrap items-center gap-3">
            <span
              v-if="(connection.active && checking) || connections.testing.value === connection.id"
              role="status"
              class="text-label text-zinc-400"
              >Checking…</span
            >
            <BackendBadge v-else-if="connection.active" :status="visibleStatus" />
            <BackendBadge
              v-else-if="connections.statuses.value[connection.id]"
              :status="connections.statuses.value[connection.id]!"
            />
            <span v-else class="text-caption text-zinc-500">Not checked</span>
            <button
              type="button"
              :disabled="checking || !!connections.testing.value"
              class="grid size-9 cursor-pointer place-items-center rounded-full text-zinc-400 hover:bg-white/8 hover:text-white disabled:opacity-40"
              :aria-label="`Check ${connection.name}`"
              @click="connection.active ? refresh() : connections.check(connection.id)"
            >
              <AppIcon name="refresh" />
            </button>
            <button
              type="button"
              class="cursor-pointer rounded-full border border-white/15 px-4 py-2 text-label text-zinc-200 hover:bg-white/5"
              @click="editing = connection"
            >
              Edit
            </button>
            <button
              v-if="!connection.active"
              type="button"
              :disabled="connections.pending.value || agent.isRunning?.value"
              class="cursor-pointer rounded-full bg-zinc-100 px-4 py-2 text-label text-zinc-950 disabled:opacity-40"
              @click="connections.activate(connection.id)"
            >
              Make active
            </button>
            <button
              type="button"
              :disabled="
                connections.pending.value ||
                (connections.catalog.value?.connections.length ?? 0) <= 1
              "
              :aria-label="`Remove ${connection.name}`"
              class="cursor-pointer px-2 py-2 text-label text-zinc-500 hover:text-white disabled:opacity-30"
              @click="removing = connection.id"
            >
              Remove
            </button>
          </div>
          <div
            v-if="removing === connection.id"
            class="flex w-full flex-wrap items-center gap-3 border-t border-white/8 pt-4 text-label text-zinc-400"
          >
            <span class="flex-1"
              >Remove {{ connection.name }} and its stored key?
              {{ connection.active ? 'Another saved connection will become active.' : '' }}</span
            >
            <button
              type="button"
              :disabled="connections.pending.value"
              class="cursor-pointer text-[#ff6b72] disabled:opacity-40"
              @click="removeConnection(connection.id)"
            >
              Confirm remove
            </button>
            <button type="button" class="cursor-pointer text-zinc-300" @click="removing = null">
              Cancel
            </button>
          </div>
        </div>
      </div>
      <p class="m-0 text-label leading-relaxed text-zinc-500">
        One connection is active at a time. Model roles are shared. Kit tools are configured under
        Tools &amp; connections.
      </p>
    </template>
    <template v-else>
      <header
        class="flex flex-wrap items-center justify-between gap-3 border-b border-white/8 pb-5"
      >
        <button
          type="button"
          class="flex cursor-pointer items-center gap-2 text-label text-zinc-400 hover:text-white"
          @click="detail = false"
        >
          <AppIcon name="back" size="sm" /> Connections
        </button>
        <span class="text-caption text-zinc-500">Model roles save automatically</span>
      </header>
      <HubGroup title="Connection">
        <HubRow label="Backend">
          <div class="max-w-96 min-w-0">
            <BackendBadge :status="checking ? null : visibleStatus" />
          </div>
          <button
            type="button"
            class="grid size-8 cursor-pointer place-items-center rounded-full border-0 bg-transparent text-zinc-300 transition hover:bg-white/8 hover:text-white"
            aria-label="Check the backend again"
            :disabled="checking"
            @click="refresh"
          >
            <AppIcon name="refresh" />
          </button>
        </HubRow>
        <HubRow label="Base URL"
          ><span class="break-all text-label text-zinc-300">{{ draft.baseUrl }}</span></HubRow
        >
        <HubRow label="API key" hint="Stored separately for this connection.">
          <span class="text-label text-zinc-400">{{
            activeConnection?.hasApiKey ? 'Configured' : 'Not set'
          }}</span>
          <button
            v-if="activeConnection"
            type="button"
            class="cursor-pointer rounded-full border border-white/15 px-4 py-2 text-label text-zinc-200"
            @click="editing = activeConnection"
          >
            Edit connection
          </button>
        </HubRow>
      </HubGroup>

      <section
        class="flex flex-col gap-3 border-t border-white/8 pt-6"
        aria-label="Discovered models"
      >
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 class="m-0 text-body text-zinc-200">Discovered models</h2>
            <p class="mt-1 mb-0 text-caption text-zinc-500">
              {{
                listAvailable
                  ? `${discovered.length} ${discovered.length === 1 ? 'model' : 'models'} reported by the server`
                  : 'Model list unavailable. Check the connection.'
              }}
            </p>
          </div>
          <button
            type="button"
            :disabled="checking"
            class="cursor-pointer text-label text-zinc-400 hover:text-white disabled:opacity-40"
            @click="refresh"
          >
            {{ checking ? 'Checking…' : 'Reload models' }}
          </button>
        </div>
        <template v-if="listAvailable && discovered.length">
          <input
            v-model="modelSearch"
            type="search"
            aria-label="Search discovered models"
            placeholder="Search models"
            class="h-10 w-full rounded-xl bg-white/5 px-3 text-label text-zinc-100 outline-none placeholder:text-zinc-500 focus-visible:ring-1 focus-visible:ring-white/30"
          />
          <ul class="m-0 max-h-56 list-none overflow-y-auto rounded-xl border border-white/8 p-0">
            <li
              v-for="model in filteredModels"
              :key="model.id"
              class="break-all border-b border-white/8 px-4 py-3 text-label text-zinc-300 last:border-0"
            >
              {{ model.id }}
            </li>
          </ul>
          <p v-if="!filteredModels.length" role="status" class="m-0 text-caption text-zinc-500">
            No matching models.
          </p>
        </template>
        <p v-else-if="listAvailable" class="m-0 text-caption text-zinc-500">
          The server reported no models.
        </p>
      </section>

      <HubGroup title="Models">
        <p class="m-0 mb-2 text-label text-zinc-400">
          Choose a model for each job. Visual Analysis requires an explicit Analysis model with
          image input and tool calling. Other optional roles can use the agent model when empty.
          Open Model capabilities to check what each model supports.
        </p>
        <datalist id="discovered-models-list">
          <option v-for="m in discovered" :key="m.id" :value="m.id" />
        </datalist>

        <ModelRoleRow
          v-model="draft.models.agent"
          role-label="Agent"
          role-key="agent"
          :agent-model="draft.models.agent"
          :models="discovered"
          placeholder="e.g. gemma-4-12b-it"
        />
        <ModelRoleRow
          v-model="draft.models.verifier"
          role-label="Verifier"
          role-key="verifier"
          :agent-model="draft.models.agent"
          :models="discovered"
        />
        <ModelRoleRow
          v-model="draft.models.analysis"
          role-label="Visual Analysis"
          role-key="analysis"
          :agent-model="draft.models.agent"
          :models="discovered"
        />
        <ModelRoleRow
          v-model="draft.models.voice"
          role-label="Voice"
          role-key="voice"
          :agent-model="draft.models.agent"
          :models="discovered"
        />
        <ModelRoleRow
          v-model="draft.models.decision"
          role-label="Decision"
          role-key="decision"
          :agent-model="draft.models.agent"
          :models="discovered"
        />
        <ModelRoleRow
          v-model="draft.models.advisor"
          role-label="Advisor"
          role-key="advisor"
          :agent-model="draft.models.agent"
          :models="discovered"
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
    <p v-if="connections.error.value" role="alert" class="m-0 text-label text-[#ff6b72]">
      {{ connections.error.value }}
    </p>
  </div>
</template>