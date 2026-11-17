<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
import RuneSwitch from './RuneSwitch.vue'
import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
import McpLibrary from './McpLibrary.vue'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref, watch } from 'vue'
defineProps<{ toolCalls?: string }>()
interface Config {
  enabled: boolean
  endpoint: string
  selected: string[]
}
interface Tool {
  name: string
  description: string
  inputSchema: unknown
}
interface KitField {
  mcp?: string
  plugin?: string
  key: string
  kind: string
  description: string
  required: boolean
  secret: boolean
  locked: boolean
  value: string
  shared?: boolean
}
interface KitRemote {
  name: string
  endpoint: string
  protocol: string
  token_env: string
}
interface KitPackage {
  name: string
  version: string
  description?: string | null
  author?: string | null
}
interface SandboxInventory {
  workspace_only: boolean
  plugins: Array<{
    plugin: string
    workspace: string | null
    approvals: Array<{ path: string; kind: string }>
  }>
}
interface ChangesInventory {
  plugins: Array<{
    plugin: string
    reviews: Array<{
      path: string
      staged_path: string
      deleted: boolean
      baseline: string | null
      staged_digest: string | null
      approval: string
    }>
    recoveries: Array<{
      path: string
      id: string
      backup_path: string
      completed: number | null
      approval: string
    }>
  }>
}
const library = ref<{ refreshTools: () => Promise<void> } | null>(null)
const folder = ref<string | null>(null)
const folderReason = ref('')
const folderLoading = ref(true)
const folderBusy = ref(false)
let folderLoad = 0
async function loadFolder() {
  const serial = ++folderLoad
  const endpoint = config.value.endpoint
  folderLoading.value = true
  folder.value = null
  folderReason.value = ''
  try {
    const value = await invoke<string | null>('kit_plugin_folder', {
      endpoint,
      plugin: 'rune-filesystem',
    })
    if (serial === folderLoad) folder.value = value
  } catch (error) {
    if (serial === folderLoad) folderReason.value = String(error)
  } finally {
    if (serial === folderLoad) folderLoading.value = false
  }
}
async function chooseFolder() {
  if (folderBusy.value || folderLoading.value || folderReason.value) return
  const endpoint = config.value.endpoint
  folderBusy.value = true
  try {
    const value = await invoke<string | null>('pick_folder')
    if (value == null || config.value.endpoint !== endpoint) return
    await invoke('kit_set_plugin_folder', {
      endpoint,
      plugin: 'rune-filesystem',
      key: 'allowed_dir',
      value,
    })
    if (config.value.endpoint !== endpoint) return
    await loadFolder()
    await refresh()
    await library.value?.refreshTools()
    if (config.value.endpoint === endpoint) status.value = 'Folder saved by Kit. ' + status.value
  } catch (error) {
    if (config.value.endpoint === endpoint) status.value = String(error)
  } finally {
    folderBusy.value = false
  }
}
const config = ref<Config>({ enabled: false, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] })
const tools = ref<Tool[]>([])
const status = ref(
  'Not connected. rune-kit 0.6.0 or newer with a compatible MCP protocol is required.',
)
const busy = ref(false)
const configReady = ref(false)
const token = ref('')
const tokenPresent = ref(false)
async function saveToken(value: string) {
  try {
    await invoke('kit_token_set', { token: value })
    token.value = ''
    tokenPresent.value = await invoke<boolean>('kit_token_present')
    status.value = tokenPresent.value ? 'Kit token is available.' : 'Kit token removed.'
  } catch (error) {
    status.value = String(error)
  }
}
const fields = ref<KitField[]>([])
const configStatus = ref('Load settings from the running Kit server.')
const configBusy = ref(false)
const remotes = ref<KitRemote[]>([])
const remoteDraft = ref<KitRemote>({
  name: '',
  endpoint: '',
  protocol: '2026-07-28',
  token_env: '',
})
const remoteStatus = ref('Load third-party servers from Kit.')
const remoteBusy = ref(false)
const sandbox = ref<SandboxInventory | null>(null)
const sandboxStatus = ref('Load sandbox information from Kit.')
const sandboxBusy = ref(false)
const changes = ref<ChangesInventory | null>(null)
const changesBusy = ref(false)
const changesStatus = ref('Load staged changes and recovery backups from Kit.')
const changeConfirmation = ref<{
  plugin: string
  action: 'apply' | 'restore'
  approval: string
  path: string
  endpoint: string
} | null>(null)
watch(
  () => config.value.endpoint,
  () => {
    if (configReady.value) void loadFolder()
    sandbox.value = null
    sandboxStatus.value = 'Load sandbox information from Kit.'
    changes.value = null
    changeConfirmation.value = null
    changesStatus.value = 'Load staged changes and recovery backups from Kit.'
  },
)
async function loadSandbox() {
  const endpoint = config.value.endpoint
  sandboxBusy.value = true
  try {
    const result = await invoke<SandboxInventory>('kit_sandbox_inventory', { config: config.value })
    if (config.value.endpoint !== endpoint) return
    sandbox.value = result
    sandboxStatus.value = sandbox.value.plugins.length
      ? 'Sandbox information provided by Kit.'
      : 'No loaded MCPs reported by Kit.'
  } catch (error) {
    if (config.value.endpoint !== endpoint) return
    sandbox.value = null
    sandboxStatus.value = String(error)
  } finally {
    sandboxBusy.value = false
  }
}
async function loadChanges() {
  const endpoint = config.value.endpoint
  changeConfirmation.value = null
  changesBusy.value = true
  try {
    const result = await invoke<ChangesInventory>('kit_changes_inventory', { config: config.value })
    if (config.value.endpoint !== endpoint) return
    changes.value = result
    changesStatus.value = 'Review information provided by Kit.'
  } catch (error) {
    if (config.value.endpoint !== endpoint) return
    changes.value = null
    changesStatus.value = String(error)
  } finally {
    changesBusy.value = false
  }
}
async function confirmChange() {
  const selected = changeConfirmation.value
  if (!selected || changesBusy.value || selected.endpoint !== config.value.endpoint) return
  changesBusy.value = true
  changeConfirmation.value = null
  try {
    await invoke('kit_change_action', {
      config: config.value,
      action: { plugin: selected.plugin, action: selected.action, approval: selected.approval },
    })
    if (config.value.endpoint !== selected.endpoint) return
    await loadChanges()
    if (config.value.endpoint === selected.endpoint) {
      const success =
        selected.action === 'apply'
          ? 'Kit applied the reviewed change.'
          : 'Kit restored the reviewed backup.'
      changesStatus.value = changes.value
        ? success
        : `${success} Could not refresh the inventory: ${changesStatus.value}`
    }
  } catch (error) {
    if (config.value.endpoint === selected.endpoint) {
      changes.value = null
      changesStatus.value = `${String(error)}. Reload and review before retrying.`
    }
  } finally {
    changesBusy.value = false
  }
}
const packages = ref<{ mcps: KitPackage[]; skills: KitPackage[] }>({ mcps: [], skills: [] })
const packageStatus = ref('Load installed packages from Kit.')
const packageBusy = ref(false)
const installKind = ref<'mcp' | 'skill'>('mcp')
const installTarget = ref('')
const installVersion = ref('')
const preferNative = ref(false)
const removePackage = ref<{ kind: 'mcp' | 'skill'; name: string } | null>(null)
async function packageAction(
  action: 'check' | 'update' | 'remove',
  kind: 'mcp' | 'skill',
  name: string,
) {
  packageBusy.value = true
  try {
    const result = await invoke<{
      has_update?: boolean
      latest_version?: string
      version?: string
      backup?: string | null
      restart_required?: boolean
    }>('kit_package_operation', { config: config.value, action, kind, name })
    if (action === 'check' || (action === 'update' && result.has_update === false)) {
      packageStatus.value = result.has_update
        ? `${name}: update available (${result.latest_version}).`
        : `${name} is up to date.`
    } else {
      removePackage.value = null
      await loadPackages()
      packageStatus.value = `${name} ${action === 'remove' ? 'removed' : `updated to ${result.version}`}.${result.restart_required ? ' Restart Kit to apply this change.' : ''}${result.backup ? ` Backup: ${result.backup}` : ''}`
    }
  } catch (error) {
    packageStatus.value = String(error)
  } finally {
    packageBusy.value = false
  }
}
async function installPackage() {
  packageBusy.value = true
  try {
    const result = await invoke<{ name: string; version: string; restart_required: boolean }>(
      'kit_install_package',
      {
        config: config.value,
        package: {
          kind: installKind.value,
          target: installTarget.value,
          version: installVersion.value || null,
          prefer_native: preferNative.value,
        },
      },
    )
    installTarget.value = ''
    installVersion.value = ''
    await loadPackages()
    packageStatus.value = `${result.name} ${result.version} installed.${result.restart_required ? ' Restart Kit to load the MCP server.' : ''}`
  } catch (error) {
    packageStatus.value = String(error)
  } finally {
    packageBusy.value = false
  }
}
async function loadPackages() {
  packageBusy.value = true
  const endpoint = config.value.endpoint
  try {
    const result = await invoke<{
      mcps?: KitPackage[]
      plugins?: KitPackage[]
      skills: KitPackage[]
    }>('kit_packages', {
      config: config.value,
    })
    if (endpoint !== config.value.endpoint) return
    const mcps = result.mcps ?? result.plugins ?? []
    packages.value = { mcps, skills: result.skills ?? [] }
    packageStatus.value = `${mcps.length} MCPs and ${packages.value.skills.length} skills installed.`
  } catch (error) {
    packages.value = { mcps: [], skills: [] }
    packageStatus.value = String(error)
  } finally {
    packageBusy.value = false
  }
}
async function loadRemotes() {
  remoteBusy.value = true
  const endpoint = config.value.endpoint
  try {
    const result = await invoke<KitRemote[]>('kit_remotes', { config: config.value })
    if (endpoint !== config.value.endpoint) return
    remotes.value = result.map((remote) => ({ ...remote, token_env: remote.token_env ?? '' }))
    remoteStatus.value = result.length
      ? 'Third-party servers are managed by Kit.'
      : 'No third-party servers configured.'
  } catch (error) {
    remotes.value = []
    packages.value = { mcps: [], skills: [] }
    packageStatus.value = 'Load installed packages from Kit.'
    remoteStatus.value = String(error)
  } finally {
    remoteBusy.value = false
  }
}
async function updateRemote(remote: KitRemote | null, name: string) {
  remoteBusy.value = true
  try {
    await invoke('kit_remote_update', {
      config: config.value,
      name,
      remote: remote && { ...remote, token_env: remote.token_env || null },
    })
    remoteDraft.value = { name: '', endpoint: '', protocol: '2026-07-28', token_env: '' }
    await loadRemotes()
  } catch (error) {
    remoteStatus.value = String(error)
  } finally {
    remoteBusy.value = false
  }
}
watch(
  () => config.value.endpoint,
  () => {
    fields.value = []
    remotes.value = []
    removePackage.value = null
    remoteDraft.value = { name: '', endpoint: '', protocol: '2026-07-28', token_env: '' }
    configStatus.value = 'Load settings from the running Kit server.'
  },
)
async function loadFields() {
  configBusy.value = true
  const endpoint = config.value.endpoint
  try {
    const result = await invoke<KitField[]>('kit_configuration', { config: config.value })
    if (config.value.endpoint !== endpoint) return
    fields.value = result
    configStatus.value = result.length
      ? 'Settings are managed by Kit.'
      : 'No MCP settings are available.'
  } catch (error) {
    fields.value = []
    configStatus.value = String(error)
  } finally {
    configBusy.value = false
  }
}
async function saveField(field: KitField) {
  configBusy.value = true
  try {
    const mcpName = field.mcp || field.plugin || ''
    await invoke('kit_update_configuration', {
      config: config.value,
      update: {
        plugin: mcpName,
        key: field.key,
        value: field.value,
        shared: field.shared ?? false,
      },
    })
    if (field.secret) field.value = ''
    configStatus.value = `${mcpName}: ${field.key} saved by Kit.`
  } catch (error) {
    configStatus.value = String(error)
  } finally {
    configBusy.value = false
  }
}
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
    const result = await invoke<{
      version: string
      tools: Tool[]
      blocked?: { name: string; reason: string }[]
    }>('kit_discover', {
      config: config.value,
    })
    tools.value = result.tools
    status.value = result.tools.length
      ? `${result.version}: ${result.tools.length} supported read-only tools available.`
      : `${result.version}: no supported tools. Check installed time/git plugins and filesystem 0.4.1+. File tools require a server-configured allowed_dir. Unsafe metadata or unsupported schemas remain unavailable.`
    if (result.blocked?.length)
      status.value += ` ${result.blocked.length} more tools are listed but cannot run yet (see the Playground).`
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
    tokenPresent.value = await invoke<boolean>('kit_token_present')
  } catch (error) {
    status.value = String(error)
  }
  configReady.value = true
  await loadFolder()
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
    <McpLibrary v-if="configReady" ref="library" v-model:config="config">
      <template #installed-details="{ plugin }">
        <HubRow
          v-if="plugin === 'rune-filesystem'"
          label="Filesystem folder"
          :hint="folderReason"
          class="w-full"
          data-testid="filesystem-folder"
        >
          <span class="min-w-0 max-w-sm break-words text-rune-caption text-rune-text-2">{{
            folder || 'No folder set'
          }}</span>
          <RuneButton
            size="sm"
            :disabled="folderLoading || folderBusy || !!folderReason"
            @click="chooseFolder"
            >{{ folderBusy ? 'Saving…' : 'Choose folder' }}</RuneButton
          >
        </HubRow>
      </template>
    </McpLibrary>
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
      <HubRow
        label="Kit access token"
        :hint="
          tokenPresent
            ? 'A token is available in the credential store or environment.'
            : 'Optional. Required when the Kit server uses authentication.'
        "
      >
        <input
          v-model="token"
          type="password"
          aria-label="Kit access token"
          autocomplete="off"
          placeholder="Enter token"
          class="h-10 w-48 rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-ink"
        />
        <RuneButton size="sm" :disabled="!token" @click="saveToken(token)">Save token</RuneButton>
        <RuneButton v-if="tokenPresent" size="sm" @click="saveToken('')"
          >Remove saved token</RuneButton
        >
      </HubRow>
      <HubRow label="Discovery" hint="Find the read-only tools this server offers.">
        <RuneButton size="sm" :disabled="busy" @click="refresh">
          {{ busy ? 'Discovering…' : 'Refresh / discover' }}
        </RuneButton>
      </HubRow>
      <template #after>
        <p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ status }}
        </p>
        <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          Change the filesystem folder here for embedded Kit; for an external Kit, set it on your
          Kit server. File contents are sent to the selected model.
        </p>
      </template>
    </HubGroup>
    <HubGroup title="Kit staged changes and backups">
      <HubRow
        label="Review inventory"
        hint="Inspect pending changes and recovery backups managed by Kit."
      >
        <RuneButton size="sm" :disabled="changesBusy" @click="loadChanges">{{
          changesBusy ? 'Loading…' : 'Load changes'
        }}</RuneButton>
      </HubRow>
      <template v-for="plugin in changes?.plugins ?? []" :key="plugin.plugin">
        <HubRow
          v-if="!plugin.reviews.length && !plugin.recoveries.length"
          :label="plugin.plugin"
          hint="No pending changes or recovery backups."
        />
        <HubRow
          v-for="review in plugin.reviews"
          :key="`${plugin.plugin}:change:${review.path}`"
          :label="`${plugin.plugin}: ${review.deleted ? 'Delete' : review.baseline ? 'Modify' : 'Create'}`"
          :hint="review.path"
        >
          <details class="text-rune-caption text-rune-text-2">
            <summary>Review details</summary>
            <div>Staged file: {{ review.staged_path }}</div>
            <div>Original SHA-256: {{ review.baseline || 'new file' }}</div>
            <div>Staged SHA-256: {{ review.staged_digest || 'delete' }}</div>
            <RuneButton
              size="sm"
              :disabled="changesBusy"
              @click="
                changeConfirmation = {
                  plugin: plugin.plugin,
                  action: 'apply',
                  approval: review.approval,
                  path: review.path,
                  endpoint: config.endpoint,
                }
              "
              >Review apply</RuneButton
            >
          </details>
        </HubRow>
        <HubRow
          v-for="recovery in plugin.recoveries"
          :key="`${plugin.plugin}:recovery:${recovery.id}`"
          :label="`${plugin.plugin}: ${recovery.completed == null ? 'Unresolved backup' : 'Completed backup'}`"
          :hint="recovery.path"
        >
          <details class="text-rune-caption text-rune-text-2">
            <summary>Backup details</summary>
            <div>Backup: {{ recovery.backup_path }}</div>
            <div>Recovery ID: {{ recovery.id }}</div>
            <RuneButton
              size="sm"
              :disabled="changesBusy"
              @click="
                changeConfirmation = {
                  plugin: plugin.plugin,
                  action: 'restore',
                  approval: recovery.approval,
                  path: recovery.path,
                  endpoint: config.endpoint,
                }
              "
              >Review restore</RuneButton
            >
          </details>
        </HubRow>
      </template>
      <HubRow
        v-if="changeConfirmation"
        :label="
          changeConfirmation.action === 'apply' ? 'Confirm file change' : 'Confirm backup restore'
        "
        :hint="changeConfirmation.path"
      >
        <RuneButton size="sm" :disabled="changesBusy" @click="confirmChange">{{
          changeConfirmation.action === 'apply' ? 'Confirm apply' : 'Confirm restore'
        }}</RuneButton>
        <RuneButton size="sm" :disabled="changesBusy" @click="changeConfirmation = null"
          >Cancel</RuneButton
        >
      </HubRow>
      <template #after
        ><p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ changesStatus }}
        </p>
        <p v-if="changes" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          File contents are not displayed. Review the details before confirming an action; Kit
          rejects stale reviews and conflicting file changes.
        </p></template
      >
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
    <HubGroup title="Kit sandbox">
      <HubRow label="Workspaces and approvals" hint="Read the current sandbox inventory from Kit.">
        <RuneButton size="sm" :disabled="sandboxBusy" @click="loadSandbox">{{
          sandboxBusy ? 'Loading…' : 'Load sandbox'
        }}</RuneButton>
      </HubRow>
      <HubRow
        v-for="plugin in sandbox?.plugins ?? []"
        :key="plugin.plugin"
        :label="plugin.plugin"
        :hint="plugin.workspace || 'Workspace not created'"
      >
        <span v-if="!plugin.approvals.length" class="text-rune-caption text-rune-text-3"
          >No path approvals</span
        >
        <ul v-else class="m-0 list-none p-0 text-rune-caption text-rune-text-2">
          <li v-for="approval in plugin.approvals" :key="approval.path">
            {{ approval.kind }}: {{ approval.path }}
          </li>
        </ul>
      </HubRow>
      <template #after>
        <p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ sandboxStatus }}
        </p>
        <p v-if="sandbox" class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
          {{
            sandbox.workspace_only
              ? 'Kit is running in workspace-only mode.'
              : 'Kit is not running in workspace-only mode.'
          }}
          Path approvals are records and do not by themselves change runtime permissions.
        </p>
      </template>
    </HubGroup>
    <HubGroup title="Kit MCP settings">
      <HubRow label="MCP configuration" hint="Read and update settings on the running Kit server.">
        <RuneButton size="sm" :disabled="configBusy" @click="loadFields">Load settings</RuneButton>
      </HubRow>
      <HubRow
        v-for="field in fields"
        :key="`${field.mcp || field.plugin}:${field.key}`"
        :label="`${field.mcp || field.plugin}: ${field.key}`"
        :hint="
          field.locked
            ? `${field.description} Managed on the Kit server.`
            : field.secret
              ? `${field.description} Secrets last for this Kit session only.`
              : field.description
        "
      >
        <template v-if="!field.locked">
          <RuneChoice
            v-if="field.kind === 'boolean'"
            v-model="field.value"
            :label="`${field.mcp || field.plugin}: ${field.key}`"
            menu
            :options="[
              { value: 'true', label: 'True' },
              { value: 'false', label: 'False' },
            ]"
          />
          <input
            v-else
            v-model="field.value"
            :type="field.secret ? 'password' : 'text'"
            :aria-label="`${field.mcp || field.plugin}: ${field.key}`"
            :placeholder="field.secret ? 'Enter a new secret' : field.kind"
            autocomplete="off"
            class="h-10 w-48 rounded-rune-tile bg-rune-sunken px-rune-2 text-rune-ink"
          />
          <label
            v-if="!field.secret"
            class="flex items-center gap-rune-2 text-rune-caption text-rune-text-2"
          >
            <RuneSwitch
              :model-value="field.shared ?? false"
              label="Shared default"
              @update:model-value="field.shared = $event"
            />
            Shared default
          </label>
          <RuneButton size="sm" :disabled="configBusy" @click="saveField(field)">Save</RuneButton>
        </template>
        <span v-else class="text-rune-caption text-rune-text-2">{{ field.value }}</span>
      </HubRow>
      <template #after
        ><p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ configStatus }}
        </p></template
      >
    </HubGroup>
    <HubGroup title="Third-party MCP servers">
      <HubRow label="Kit servers" hint="Kit connects to and manages these servers.">
        <RuneButton size="sm" :disabled="remoteBusy" @click="loadRemotes">Load servers</RuneButton>
      </HubRow>
      <HubRow
        v-for="remote in remotes"
        :key="remote.name"
        :label="remote.name"
        :hint="`${remote.endpoint} · ${remote.protocol}`"
      >
        <RuneButton
          size="sm"
          :disabled="remoteBusy"
          @click="remoteDraft = { ...remote, endpoint: '' }"
          >Edit</RuneButton
        >
        <RuneButton size="sm" :disabled="remoteBusy" @click="updateRemote(null, remote.name)"
          >Remove</RuneButton
        >
      </HubRow>
      <HubInput
        v-model="remoteDraft.name"
        label="Server name"
        hint="Lowercase letters, numbers, hyphens and underscores."
      />
      <HubInput
        v-model="remoteDraft.endpoint"
        label="Server endpoint"
        hint="Enter the full endpoint when adding or editing; Kit hides query parameters in the list."
      />
      <HubInput v-model="remoteDraft.protocol" label="MCP protocol version" />
      <HubInput
        v-model="remoteDraft.token_env"
        label="Token environment variable"
        hint="Optional variable available to the Kit server."
      />
      <HubRow label="Save server"
        ><RuneButton
          size="sm"
          :disabled="remoteBusy || !remoteDraft.name || !remoteDraft.endpoint"
          @click="updateRemote(remoteDraft, remoteDraft.name)"
          >Save server</RuneButton
        ></HubRow
      >
      <template #after
        ><p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ remoteStatus }}
        </p></template
      >
    </HubGroup>
    <HubGroup title="Skills and custom packages">
      <HubRow
        label="Installed skills"
        hint="Skills installed in Kit’s shared storage. Manage MCP servers above."
      >
        <RuneButton size="sm" :disabled="packageBusy" @click="loadPackages"
          >Load packages</RuneButton
        >
      </HubRow>
      <HubRow label="Package type">
        <RuneChoice
          v-model="installKind"
          label="Package type"
          menu
          :options="[
            { value: 'mcp', label: 'MCP Server' },
            { value: 'skill', label: 'Skill' },
          ]"
        />
      </HubRow>
      <HubInput
        v-model="installTarget"
        label="Package target"
        hint="Registry name, HTTPS URL, or absolute local path."
      />
      <HubInput
        v-model="installVersion"
        label="Package version"
        hint="Optional; leave blank for the default version."
      />
      <HubToggle
        v-if="installKind === 'mcp'"
        v-model="preferNative"
        label="Prefer native MCP binary"
      />
      <HubRow label="Install package"
        ><RuneButton
          size="sm"
          :disabled="packageBusy || !installTarget.trim()"
          @click="installPackage"
          >Install</RuneButton
        ></HubRow
      >
      <HubRow
        v-for="item in packages.skills"
        :key="`skill:${item.name}`"
        :label="item.name"
        :hint="`Skill · ${item.version}${item.author ? ` · ${item.author}` : ''}${item.description ? ` · ${item.description}` : ''}`"
      >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="packageAction('check', 'skill', item.name)"
          >Check update</RuneButton
        >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="packageAction('update', 'skill', item.name)"
          >Update</RuneButton
        >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="removePackage = { kind: 'skill', name: item.name }"
          >Remove</RuneButton
        >
      </HubRow>
      <HubRow
        v-if="removePackage"
        :label="`Remove ${removePackage.name}?`"
        hint="This removes the installed package and its files."
      >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="packageAction('remove', removePackage.kind, removePackage.name)"
          >Confirm removal</RuneButton
        >
        <RuneButton size="sm" :disabled="packageBusy" @click="removePackage = null"
          >Cancel</RuneButton
        >
      </HubRow>
      <template #after
        ><p role="status" class="m-0 mt-rune-1 px-rune-2 text-rune-caption text-rune-text-2">
          {{ packageStatus }}
        </p></template
      >
    </HubGroup>
  </div>
</template>