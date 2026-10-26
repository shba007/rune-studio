<script setup lang="ts">
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
    <div class="flex items-center gap-3 border-b border-white/10 pb-5 text-label text-zinc-400">
      <PersonalitySymbol mark="companion" class="size-8 shrink-0 text-zinc-200" />
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
            class="flex cursor-pointer items-center gap-3 rounded-xl px-3 py-3 text-left text-label transition disabled:opacity-40"
            :class="
              draft?.id === profile.id
                ? 'bg-white/10 text-zinc-100'
                : 'text-zinc-400 hover:bg-white/5'
            "
            :aria-pressed="draft?.id === profile.id"
            @click="edit(profile)"
          >
            <span class="grid size-9 shrink-0 place-items-center rounded-xl bg-white/5"
              ><PersonalitySymbol :mark="profile.mark" class="size-5"
            /></span>
            <span class="min-w-0"
              ><span class="block truncate">{{ profile.name }}</span
              ><span class="mt-1 block text-caption text-zinc-500">{{
                profile.id === 'base'
                  ? 'Default companion'
                  : profile.categories.join(' · ') || 'No categories'
              }}</span></span
            >
          </button>
        </nav>
        <details class="border-t border-white/10 pt-4 text-label text-zinc-400">
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
                class="min-w-0 rounded-lg bg-white/5 p-3 text-zinc-100"
            /></label>
            <p class="m-0 text-caption leading-relaxed text-zinc-500">
              Only the category is sent to a verified local model. Review before saving.
            </p>
            <button
              type="submit"
              :disabled="busy || !suggestionCategory.trim()"
              class="cursor-pointer rounded-full border border-white/15 px-3 py-2 text-zinc-200 disabled:opacity-40"
            >
              {{ generating ? 'Drafting…' : 'Suggest personality' }}
            </button>
          </form>
        </details>
      </aside>
      <div class="min-w-0">
        <p v-if="suggestion" role="status" class="mb-5 text-label text-zinc-300">
          Unsaved suggestion · review and approve below.
        </p>
        <form
          v-if="draft"
          aria-label="Edit personality"
          class="flex flex-col gap-5 text-label"
          @submit.prevent="save"
        >
          <header class="flex items-center gap-3">
            <span class="grid size-12 place-items-center rounded-2xl bg-white/5"
              ><PersonalitySymbol :mark="draft.mark" class="size-6 text-zinc-100"
            /></span>
            <div>
              <h2 class="m-0 text-body text-zinc-100">
                {{ expected ? draft.name : 'New personality' }}
              </h2>
              <p class="mt-1 mb-0 text-caption text-zinc-500">
                {{
                  draft.id === 'base'
                    ? 'Fallback for every unassigned goal'
                    : 'Communication style and goal categories'
                }}
              </p>
            </div>
          </header>
          <fieldset :disabled="busy" class="m-0 flex min-w-0 flex-col gap-5 border-0 p-0">
            <div class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_180px]">
              <label class="flex min-w-0 flex-col gap-2 text-zinc-400"
                >Name<input
                  v-model="draft.name"
                  required
                  maxlength="80"
                  class="min-w-0 rounded-xl bg-white/5 p-3 text-zinc-100"
              /></label>
              <label class="flex min-w-0 flex-col gap-2 text-zinc-400"
                >Orb symbol<select
                  v-model="draft.mark"
                  class="min-w-0 rounded-xl bg-[#1b1b1b] p-3 text-zinc-100"
                >
                  <option v-for="symbol in personalitySymbols" :key="symbol.id" :value="symbol.id">
                    {{ symbol.label }}
                  </option>
                </select></label
              >
            </div>
            <label v-if="!expected" class="flex flex-col gap-2 text-zinc-400"
              >File ID<input
                v-model="draft.id"
                required
                pattern="[a-z0-9-]+"
                maxlength="64"
                placeholder="fitness-coach"
                class="rounded-xl bg-white/5 p-3 text-zinc-100"
            /></label>
            <label v-if="draft.id !== 'base'" class="flex flex-col gap-2 text-zinc-400"
              >Goal categories<input
                v-model="categories"
                placeholder="fitness, running"
                class="rounded-xl bg-white/5 p-3 text-zinc-100"
            /></label>
            <label class="flex flex-col gap-2 text-zinc-400"
              >Communication style<textarea
                v-model="draft.prompt"
                required
                rows="8"
                maxlength="8000"
                class="resize-y rounded-xl bg-white/5 p-4 text-body leading-relaxed text-zinc-100"
              />
            </label>
            <div class="flex flex-wrap items-center gap-4 border-t border-white/10 pt-5">
              <button
                type="submit"
                class="cursor-pointer rounded-full bg-zinc-100 px-5 py-2 text-zinc-950"
              >
                {{ pending ? 'Saving…' : 'Approve and save' }}
              </button>
              <button
                type="button"
                class="cursor-pointer text-zinc-400 hover:text-white"
                @click="load"
              >
                Reload saved version
              </button>
            </div>
          </fieldset>
        </form>
        <details class="mt-6 text-caption leading-relaxed text-zinc-500">
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
    <p v-if="error" role="alert" class="m-0 text-label text-[#ff6b72]">
      {{ error }} Your draft is preserved.
    </p>
    <p v-if="saved" role="status" class="m-0 text-label text-zinc-300">
      Personality saved. New model turns use the approved style.
    </p>
  </div>
</template>