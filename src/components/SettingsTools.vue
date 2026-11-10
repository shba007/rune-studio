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
    tokenPresent.value = await invoke<boolean>('kit_token_present')
  } catch (error) {
    status.value = String(error)
  }
  configReady.value = true
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
    <McpLibrary v-if="configReady" v-model:config="config" />
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
    <HubGroup title="Installed Kit packages">
      <HubRow label="MCPs and skills" hint="Packages installed in Kit’s shared storage.">
        <RuneButton size="sm" :disabled="packageBusy" @click="loadPackages"
          >Load packages</RuneButton
        >
      </HubRow>
      <HubRow
        v-for="item in packages.mcps"
        :key="`mcp:${item.name}`"
        :label="item.name"
        :hint="`MCP · ${item.version}${item.description ? ` · ${item.description}` : ''}`"
      >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="packageAction('check', 'mcp', item.name)"
          >Check update</RuneButton
        >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="packageAction('update', 'mcp', item.name)"
          >Update</RuneButton
        >
        <RuneButton
          size="sm"
          :disabled="packageBusy"
          @click="removePackage = { kind: 'mcp', name: item.name }"
          >Remove</RuneButton
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