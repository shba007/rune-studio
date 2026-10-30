<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { DOMAINS, domainIcon } from '@/config/domains'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
import RuneIconButton from './RuneIconButton.vue'
import RuneNotice from './RuneNotice.vue'

const emit = defineEmits<{ close: []; done: [count: number] }>()
const { showSanctuary } = useDomainPrefs()

const domains = computed(() => DOMAINS.filter((d) => showSanctuary.value || d.zone === 'engine'))
const index = ref(0)
const created = ref(0)
const mode = ref<'manual' | 'auto'>('manual')
const text = ref('')
const autoReady = ref(false)
const pending = ref(false)
const error = ref('')

const domain = computed(() => domains.value[index.value]!)
const last = computed(() => index.value === domains.value.length - 1)
const modes = computed(() => [
  { value: 'auto' as const, label: 'Auto', disabled: !autoReady.value },
  { value: 'manual' as const, label: 'Manual' },
])
const ready = computed(() => (mode.value === 'auto' ? autoReady.value : !!text.value.trim()))

onMounted(async () => {
  try {
    autoReady.value = (await invoke<string[]>('workspace_goal_modes')).includes('auto')
    if (autoReady.value) mode.value = 'auto'
  } catch {
    autoReady.value = false
  }
})

function changeMode(value: 'manual' | 'auto') {
  if (value === 'auto' && !autoReady.value) return
  mode.value = value
}

function next() {
  text.value = ''
  error.value = ''
  if (last.value) emit('done', created.value)
  else index.value += 1
}

async function add() {
  if (pending.value || !ready.value) return
  pending.value = true
  error.value = ''
  try {
    await invoke<string>(
      'workspace_new_goal',
      mode.value === 'auto'
        ? { text: null, domain: domain.value.id, mode: 'auto', subDomains: [] }
        : { text: text.value, domain: domain.value.id, mode: 'manual', subDomains: [] },
    )
    created.value += 1
    next()
  } catch (e) {
    error.value = String(e)
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <section
    role="dialog"
    aria-label="Set up your life domains"
    class="glass absolute inset-0 z-20 grid place-items-center bg-black/40 p-rune-2"
  >
    <form
      class="flex w-full max-w-md flex-col gap-rune-2 rounded-rune-tile bg-rune-surface p-rune-3"
      @submit.prevent="add"
    >
      <header class="flex items-center justify-between gap-rune-2">
        <span class="text-rune-heading text-rune-text-2 uppercase">
          Step {{ index + 1 }} of {{ domains.length }}
        </span>
        <RuneIconButton variant="ghost" size="sm" label="Close setup" @click="emit('close')">
          ×
        </RuneIconButton>
      </header>
      <div class="flex items-center gap-rune-1">
        <span class="grid size-12 place-items-center rounded-full bg-rune-raised text-rune-ink">
          <AppIcon :name="domainIcon(domain.id)" />
        </span>
        <h2 class="m-0 font-sans text-rune-subtitle text-rune-ink">{{ domain.name }}</h2>
      </div>
      <fieldset :disabled="pending" class="m-0 flex min-w-0 flex-col gap-rune-2 p-0">
        <RuneChoice
          :model-value="mode"
          label="How should we start?"
          :options="modes"
          @update:model-value="changeMode"
        />
        <p v-if="!autoReady" class="m-0 text-rune-caption text-rune-text-3">
          Rune-asked questions need a Rune update that is not installed yet.
        </p>
        <label
          v-if="mode === 'manual'"
          class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
          >What would you like to achieve in {{ domain.name }}?<textarea
            v-model="text"
            rows="3"
            maxlength="4000"
            class="min-w-0 resize-y rounded-rune-tile bg-rune-sunken p-rune-1 font-rune-reading text-rune-body text-rune-ink [color-scheme:dark]"
          />
        </label>
        <p v-else class="m-0 text-rune-body text-rune-text-2">
          Rune will ask a few questions about {{ domain.name }} and suggest a first goal.
        </p>
        <div class="flex flex-wrap items-center justify-between gap-rune-1">
          <RuneButton type="button" variant="ghost" @click="next">
            {{ last ? 'Finish' : 'Skip' }}
          </RuneButton>
          <RuneButton variant="primary" type="submit" :disabled="pending || !ready">
            {{
              pending
                ? 'Starting…'
                : mode === 'auto'
                  ? `Start ${domain.name} discovery`
                  : last
                    ? 'Add goal & finish'
                    : 'Add goal & next'
            }}
          </RuneButton>
        </div>
      </fieldset>
      <RuneNotice v-if="error" tone="error">{{ error }}</RuneNotice>
    </form>
  </section>
</template>