<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { DOMAINS, domainById, suggestDomain } from '@/config/domains'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import HubRow from './HubRow.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
import RuneIconButton from './RuneIconButton.vue'
import RuneNotice from './RuneNotice.vue'
import RuneSwitch from './RuneSwitch.vue'

const emit = defineEmits<{ close: []; saved: [kind: 'goal', id?: string] }>()
const { showSanctuary, overrides, subDomains } = useDomainPrefs()

const mode = ref<'manual' | 'auto'>('manual')
const text = ref('')
const domain = ref('')
const picked = ref<string[]>([])
const autoReady = ref(false)
const pending = ref(false)
const error = ref('')

const visible = computed(() =>
  DOMAINS.filter((item) => showSanctuary.value || item.zone === 'engine'),
)
const suggestion = computed(() => (mode.value === 'manual' ? suggestDomain(text.value) : null))
const hiddenSuggestion = computed(
  () => !!suggestion.value && !visible.value.some((item) => item.id === suggestion.value!.id),
)
const effective = computed(() => {
  if (domain.value) return domainById(domain.value)
  return suggestion.value && !hiddenSuggestion.value ? suggestion.value : null
})
const domainOptions = computed(() => [
  {
    value: '',
    label:
      mode.value === 'auto'
        ? 'Choose a domain'
        : `Suggested · ${effective.value?.name ?? 'Unassigned'}`,
  },
  ...visible.value.map((item) => ({ value: item.id, label: item.name })),
])
const modes = computed(() => [
  { value: 'manual' as const, label: 'Manual' },
  { value: 'auto' as const, label: 'Auto', disabled: !autoReady.value },
])
const ready = computed(() =>
  mode.value === 'manual' ? !!text.value.trim() : !!domain.value && autoReady.value,
)
const field =
  'min-w-0 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3 [color-scheme:dark]'

function toggle(id: string, on: boolean) {
  picked.value = on ? [...new Set([...picked.value, id])] : picked.value.filter((x) => x !== id)
}
function changeDomain(value: string) {
  domain.value = value
  picked.value = []
}
function changeMode(value: 'manual' | 'auto') {
  mode.value = value
  if (value === 'auto' && !domain.value) domain.value = visible.value[0]?.id ?? ''
  if (value === 'manual') picked.value = []
}

onMounted(async () => {
  try {
    autoReady.value = (await invoke<string[]>('workspace_goal_modes')).includes('auto')
    if (autoReady.value) changeMode('auto')
  } catch {
    autoReady.value = false
  }
})

async function submit() {
  if (pending.value || !ready.value) return
  pending.value = true
  error.value = ''
  try {
    const id =
      mode.value === 'auto'
        ? await invoke<string>('workspace_new_goal', {
            text: null,
            domain: domain.value,
            mode: 'auto',
            subDomains: picked.value,
          })
        : await invoke<string>('workspace_new_goal', { text: text.value })
    if (effective.value && (domain.value || mode.value === 'auto'))
      overrides.value = { ...overrides.value, [id]: effective.value.id }
    if (picked.value.length) subDomains.value = { ...subDomains.value, [id]: picked.value }
    emit('saved', 'goal', id)
  } catch (e) {
    error.value = String(e)
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <form
    aria-label="Create a goal"
    class="flex flex-col gap-rune-2 rounded-rune-tile bg-rune-sunken p-rune-2"
    @submit.prevent="submit"
  >
    <header class="flex items-center justify-between gap-rune-2">
      <h2 class="m-0 font-sans text-rune-subtitle text-rune-ink">New goal</h2>
      <RuneIconButton
        variant="ghost"
        size="sm"
        :disabled="pending"
        label="Close creation form"
        @click="emit('close')"
      >
        ×
      </RuneIconButton>
    </header>
    <fieldset :disabled="pending" class="m-0 flex min-w-0 flex-col gap-rune-2 p-0">
      <HubRow
        label="How should we start?"
        hint="Write the goal yourself, or pick a domain and let Rune ask questions to decide the goal for you."
      >
        <RuneChoice
          :model-value="mode"
          label="How should we start?"
          :options="modes"
          @update:model-value="changeMode"
        />
      </HubRow>
      <p
        v-if="!autoReady"
        class="m-0 px-rune-2 text-rune-caption text-rune-text-3"
        data-testid="auto-note"
      >
        Rune-asked questions need a Rune update that is not installed yet.
      </p>
      <label
        v-if="mode === 'manual'"
        class="flex flex-col gap-1 rounded-rune-tile bg-rune-surface p-rune-2 text-rune-caption text-rune-text-3"
        >What would you like to achieve?<textarea
          v-model="text"
          rows="2"
          maxlength="4000"
          required
          :class="[field, 'resize-y py-rune-1']"
        />
      </label>
      <div class="flex flex-col gap-0.5">
        <HubRow label="Domain">
          <RuneChoice
            :model-value="domain"
            label="Domain"
            menu
            :options="domainOptions"
            @update:model-value="changeDomain"
          />
        </HubRow>
        <p
          v-if="hiddenSuggestion"
          class="m-0 rounded-b-rune-tile bg-rune-surface px-rune-2 py-rune-1 text-rune-caption text-rune-text-3"
        >
          This looks like a {{ suggestion!.name }} goal. Sanctuary domains are hidden; turn them on
          in Settings to file it there.
        </p>
      </div>
      <div v-if="effective" class="flex flex-col gap-0.5" data-testid="sub-domains">
        <HubRow
          v-for="item in effective.subDomains"
          :key="item.id"
          :label="`${item.number} ${item.name}`"
          :hint="
            item.id === effective.subDomains[0]?.id
              ? 'Optional. Choose the parts that matter; Rune decides when none are chosen.'
              : undefined
          "
        >
          <RuneSwitch
            :model-value="picked.includes(item.id)"
            :label="item.name"
            @update:model-value="(on: boolean) => toggle(item.id, on)"
          />
        </HubRow>
      </div>
      <RuneButton variant="primary" type="submit" :disabled="pending || !ready" class="self-start">
        {{
          pending
            ? 'Starting…'
            : mode === 'auto'
              ? 'Let Rune ask questions'
              : 'Start goal conversation'
        }}
      </RuneButton>
    </fieldset>
    <RuneNotice v-if="error" tone="error">{{ error }}</RuneNotice>
  </form>
</template>