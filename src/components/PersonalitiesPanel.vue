<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { Personality, PersonalityCatalog } from '@/types/personalities'
import { personalitySymbol, personalitySymbols } from '@/config/personalitySymbols'
import PersonalitySymbol from './PersonalitySymbol.vue'
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
defineExpose({ create })
</script>
<template>
  <div class="flex flex-col gap-6">
    <div class="flex items-center gap-3 pb-5 text-rune-caption text-rune-text-2">
      <PersonalitySymbol mark="companion" class="size-8 shrink-0 text-rune-ink" />
      <p class="m-0 max-w-xl leading-relaxed">
        A different voice for each goal. Unassigned categories use Rune.
      </p>
    </div>
    <div class="grid items-start gap-8 lg:grid-cols-[200px_minmax(0,1fr)]">
      <aside class="flex flex-col gap-5">
        <nav aria-label="Saved personalities" class="flex flex-wrap gap-2 lg:flex-col">
          <button
            v-for="profile in catalog?.entries"
            :key="profile.id"
            :disabled="busy"
            class="flex cursor-pointer items-center gap-3 rounded-rune-tile px-3 py-3 text-left text-rune-caption transition disabled:opacity-40"
            :class="
              draft?.id === profile.id
                ? 'bg-rune-raised text-rune-ink'
                : 'text-rune-text-2 hover:bg-white/5'
            "
            :aria-pressed="draft?.id === profile.id"
            @click="edit(profile)"
          >
            <span class="grid size-9 shrink-0 place-items-center rounded-rune-tile bg-rune-surface"
              ><PersonalitySymbol :mark="profile.mark" class="size-5"
            /></span>
            <span class="min-w-0"
              ><span class="block truncate">{{ profile.name }}</span
              ><span class="mt-1 block text-rune-heading text-rune-text-2">{{
                profile.id === 'base'
                  ? 'Default companion'
                  : profile.categories.join(' · ') || 'No categories'
              }}</span></span
            >
          </button>
        </nav>
        <details class="pt-4 text-rune-caption text-rune-text-2">
          <summary class="cursor-pointer">Draft with Rune</summary>
          <form
            class="mt-4 flex flex-col gap-3"
            aria-label="Suggest a personality"
            @submit.prevent="suggest"
          >
            <label class="flex flex-col gap-2"
              >Goal category<input
                v-model="suggestionCategory"
                :disabled="busy"
                maxlength="80"
                required
                placeholder="Fitness, learning…"
                class="min-w-0 rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
            /></label>
            <p class="m-0 text-rune-heading leading-relaxed text-rune-text-2">
              Only the category is sent to a verified local model. Review before saving.
            </p>
            <RuneButton type="submit" :disabled="busy || !suggestionCategory.trim()">
              {{ generating ? 'Drafting…' : 'Suggest personality' }}
            </RuneButton>
          </form>
        </details>
      </aside>
      <div class="min-w-0">
        <p v-if="suggestion" role="status" class="mb-5 text-rune-caption text-rune-text-2">
          Unsaved suggestion · review and approve below.
        </p>
        <form
          v-if="draft"
          aria-label="Edit personality"
          class="flex flex-col gap-5 text-rune-caption"
          @submit.prevent="save"
        >
          <header class="flex items-center gap-3">
            <span class="grid size-12 place-items-center rounded-rune-tile bg-rune-surface"
              ><PersonalitySymbol :mark="draft.mark" class="size-6 text-rune-ink"
            /></span>
            <div>
              <h2 class="m-0 text-rune-body text-rune-ink">
                {{ expected ? draft.name : 'New personality' }}
              </h2>
              <p class="mt-1 mb-0 text-rune-heading text-rune-text-2">
                {{
                  draft.id === 'base'
                    ? 'Fallback for every unassigned goal'
                    : 'Communication style and goal categories'
                }}
              </p>
            </div>
          </header>
          <fieldset :disabled="busy" class="m-0 flex min-w-0 flex-col gap-5 p-0">
            <div class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_180px]">
              <label class="flex min-w-0 flex-col gap-2 text-rune-text-2"
                >Name<input
                  v-model="draft.name"
                  required
                  maxlength="80"
                  class="min-w-0 rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
              /></label>
              <label class="flex min-w-0 flex-col gap-2 text-rune-text-2"
                >Orb symbol<select
                  v-model="draft.mark"
                  class="min-w-0 rounded-rune-tile bg-rune-raised p-3 text-rune-ink"
                >
                  <option v-for="symbol in personalitySymbols" :key="symbol.id" :value="symbol.id">
                    {{ symbol.label }}
                  </option>
                </select></label
              >
            </div>
            <label v-if="!expected" class="flex flex-col gap-2 text-rune-text-2"
              >File ID<input
                v-model="draft.id"
                required
                pattern="[a-z0-9-]+"
                maxlength="64"
                placeholder="fitness-coach"
                class="rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
            /></label>
            <label v-if="draft.id !== 'base'" class="flex flex-col gap-2 text-rune-text-2"
              >Goal categories<input
                v-model="categories"
                placeholder="fitness, running"
                class="rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
            /></label>
            <label class="flex flex-col gap-2 text-rune-text-2"
              >Communication style<textarea
                v-model="draft.prompt"
                required
                rows="8"
                maxlength="8000"
                class="resize-y rounded-rune-tile bg-rune-surface p-4 text-rune-body leading-relaxed text-rune-ink"
              />
            </label>
            <div class="flex flex-wrap items-center gap-4 pt-5">
              <RuneButton variant="primary" type="submit">
                {{ pending ? 'Saving…' : 'Approve and save' }}
              </RuneButton>
              <RuneButton variant="ghost" size="sm" @click="load">
                Reload saved version
              </RuneButton>
            </div>
          </fieldset>
        </form>
        <details class="mt-6 text-rune-heading leading-relaxed text-rune-text-2">
          <summary class="cursor-pointer">Storage &amp; boundaries</summary>
          <p>
            Styles change communication. Execution permissions, consent and output contracts remain
            protected. Saving approves the style and category assignments. Keep personal records in
            protected goal conversations.
          </p>
          <p v-if="catalog" class="select-text [overflow-wrap:anywhere]">
            Prompt folder: {{ catalog.folder }}
          </p>
        </details>
      </div>
    </div>
    <p v-if="error" role="alert" class="m-0 text-rune-caption text-error">
      {{ error }} Your draft is preserved.
    </p>
    <p v-if="saved" role="status" class="m-0 text-rune-caption text-rune-text-2">
      Personality saved. New model turns use the approved style.
    </p>
  </div>
</template>