<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { Personality, PersonalityCatalog } from '@/types/personalities'
import { personalitySymbol, personalitySymbols } from '@/config/personalitySymbols'
import PersonalitySymbol from './PersonalitySymbol.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import RuneChoice from './RuneChoice.vue'
import RuneNotice from './RuneNotice.vue'
import WorkRow from './WorkRow.vue'
const symbolOptions = personalitySymbols.map((symbol) => ({
  value: symbol.id,
  label: symbol.label,
}))
const field =
  'h-10 min-w-0 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3'
const catalog = ref<PersonalityCatalog | null>(null)
const draft = ref<Personality | null>(null)
const expected = ref<Personality | null>(null)
const categories = ref('')
const error = ref('')
const pending = ref(false)
const generating = ref(false)
const busy = computed(() => pending.value || generating.value)
const suggestionCategory = ref('')
const suggestion = ref(false)
let disposed = false
const saved = ref(false)
function edit(profile: Personality) {
  expected.value = { ...profile, categories: [...profile.categories] }
  draft.value = {
    ...profile,
    mark: personalitySymbol(profile.mark).id,
    categories: [...profile.categories],
  }
  categories.value = profile.categories.join(', ')
  error.value = ''
  saved.value = false
  suggestion.value = false
}
async function load() {
  try {
    const next = await invoke<PersonalityCatalog>('personalities_state')
    catalog.value = next
    const profile = next.entries.find((p) => p.id === draft.value?.id) ?? next.entries[0]
    if (profile) edit(profile)
  } catch (e) {
    error.value = String(e)
  }
}
function create() {
  if (busy.value) return
  expected.value = null
  draft.value = { id: '', name: '', mark: 'companion', categories: [], prompt: '' }
  categories.value = ''
  error.value = ''
  saved.value = false
  suggestion.value = false
}
async function suggest() {
  if (busy.value || !suggestionCategory.value.trim()) return
  generating.value = true
  error.value = ''
  try {
    const profile = await invoke<Personality>('personality_propose', {
      category: suggestionCategory.value.trim(),
    })
    if (disposed) return
    edit(profile)
    expected.value = null
    suggestion.value = true
  } catch (e) {
    if (!disposed) error.value = String(e)
  } finally {
    generating.value = false
  }
}
async function save() {
  if (!draft.value || busy.value) return
  pending.value = true
  error.value = ''
  try {
    const profile = {
      ...draft.value,
      categories:
        draft.value.id === 'base'
          ? []
          : [
              ...new Set(
                categories.value
                  .split(',')
                  .map((s) => s.trim())
                  .filter(Boolean),
              ),
            ],
    }
    catalog.value = await invoke<PersonalityCatalog>('personality_save', {
      profile,
      expected: expected.value,
    })
    edit(profile)
    saved.value = true
  } catch (e) {
    error.value = String(e)
  } finally {
    pending.value = false
  }
}
onMounted(load)
onBeforeUnmount(() => {
  disposed = true
})
const open = (profile: Personality) => {
  if (!busy.value) edit(profile)
}
defineExpose({ create })
</script>
<template>
  <div class="flex flex-col gap-rune-3">
    <p class="m-0 px-rune-2 text-rune-caption text-rune-text-3">
      A different voice for each goal. Unassigned categories use Rune.
    </p>
    <HubGroup v-if="catalog?.entries.length" title="Saved personalities">
      <WorkRow
        v-for="profile in catalog.entries"
        :key="profile.id"
        :title="profile.name"
        :meta="
          profile.id === 'base'
            ? 'Default companion'
            : profile.categories.join(' · ') || 'No categories'
        "
        :selected="draft?.id === profile.id"
        interactive
        @open="open(profile)"
      >
        <template #leading>
          <span class="grid size-10 shrink-0 place-items-center rounded-rune-tile bg-rune-sunken"
            ><PersonalitySymbol :mark="profile.mark" class="size-5 text-rune-ink"
          /></span>
        </template>
      </WorkRow>
    </HubGroup>
    <details class="rounded-rune-tile bg-rune-surface text-rune-caption text-rune-text-2">
      <summary class="cursor-pointer px-rune-2 py-rune-2 text-rune-body hover:text-rune-ink">
        Draft with Rune
      </summary>
      <form
        class="flex flex-col gap-rune-2 px-rune-2 pb-rune-2"
        aria-label="Suggest a personality"
        @submit.prevent="suggest"
      >
        <label class="flex flex-col gap-1 text-rune-text-3"
          >Goal category<input
            v-model="suggestionCategory"
            :disabled="busy"
            maxlength="80"
            required
            placeholder="Fitness, learning…"
            :class="field"
        /></label>
        <p class="m-0 text-rune-text-3">
          Only the category is sent to a verified local model. Review before saving.
        </p>
        <RuneButton
          variant="secondary"
          size="sm"
          type="submit"
          class="self-start"
          :disabled="busy || !suggestionCategory.trim()"
        >
          {{ generating ? 'Drafting…' : 'Suggest personality' }}
        </RuneButton>
      </form>
    </details>
    <RuneNotice v-if="suggestion">Unsaved suggestion · review and approve below.</RuneNotice>
    <form
      v-if="draft"
      aria-label="Edit personality"
      class="flex flex-col gap-rune-3"
      @submit.prevent="save"
    >
      <fieldset :disabled="busy" class="m-0 flex min-w-0 flex-col gap-rune-3 p-0">
        <HubGroup
          :title="expected ? draft.name : 'New personality'"
          :hint="
            draft.id === 'base'
              ? 'Fallback for every unassigned goal'
              : 'Communication style and goal categories'
          "
        >
          <HubRow label="Name">
            <input
              v-model="draft.name"
              required
              maxlength="80"
              aria-label="Name"
              :class="[field, 'w-full sm:w-72']"
            />
          </HubRow>
          <HubRow label="Orb symbol">
            <RuneChoice v-model="draft.mark" label="Orb symbol" :options="symbolOptions" menu />
          </HubRow>
          <HubRow v-if="!expected" label="File ID" hint="Lowercase letters, numbers and dashes">
            <input
              v-model="draft.id"
              required
              pattern="[a-z0-9-]+"
              maxlength="64"
              placeholder="fitness-coach"
              aria-label="File ID"
              :class="[field, 'w-full sm:w-72']"
            />
          </HubRow>
          <HubRow v-if="draft.id !== 'base'" label="Goal categories" hint="Separate with commas">
            <input
              v-model="categories"
              placeholder="fitness, running"
              aria-label="Goal categories"
              :class="[field, 'w-full sm:w-72']"
            />
          </HubRow>
        </HubGroup>
        <HubGroup title="Communication style">
          <label
            class="flex flex-col gap-1 rounded-rune-tile bg-rune-surface p-rune-2 text-rune-caption text-rune-text-3"
          >
            <span class="sr-only">Communication style</span>
            <textarea
              v-model="draft.prompt"
              required
              rows="8"
              maxlength="8000"
              class="resize-y rounded-rune-tile bg-rune-sunken p-rune-2 font-rune-reading text-rune-body text-rune-ink"
            />
          </label>
        </HubGroup>
        <div class="flex flex-wrap items-center gap-rune-1">
          <RuneButton variant="primary" type="submit">
            {{ pending ? 'Saving…' : 'Approve and save' }}
          </RuneButton>
          <RuneButton variant="ghost" size="sm" @click="load"> Reload saved version </RuneButton>
        </div>
      </fieldset>
    </form>
    <details class="text-rune-caption text-rune-text-3">
      <summary class="w-fit cursor-pointer px-rune-2 py-1 hover:text-rune-ink">
        Storage &amp; boundaries
      </summary>
      <p class="mx-rune-2 mt-rune-1 mb-0">
        Styles change communication. Execution permissions, consent and output contracts remain
        protected. Saving approves the style and category assignments. Keep personal records in
        protected goal conversations.
      </p>
      <p v-if="catalog" class="mx-rune-2 mt-rune-1 mb-0 select-text [overflow-wrap:anywhere]">
        Prompt folder: {{ catalog.folder }}
      </p>
    </details>
    <RuneNotice v-if="error" tone="error">{{ error }} Your draft is preserved.</RuneNotice>
    <RuneNotice v-if="saved">Personality saved. New model turns use the approved style.</RuneNotice>
  </div>
</template>