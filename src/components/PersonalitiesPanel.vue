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
</script>
<template>
  <div class="flex flex-col gap-5">
    <p class="m-0 text-label text-zinc-400">
      Personalities change how Rune communicates. Execution permissions, consent, and output
      contracts remain protected. Unassigned categories use the base personality.
    </p>
    <p v-if="catalog" class="m-0 text-caption text-zinc-500 select-text [overflow-wrap:anywhere]">
      Prompt folder: {{ catalog.folder }}
    </p>
    <form
      class="flex max-w-2xl flex-col gap-3 rounded-2xl border border-white/10 p-4"
      aria-label="Suggest a personality"
      @submit.prevent="suggest"
    >
      <label class="flex flex-col gap-2 text-label text-zinc-300"
        >Goal category<input
          v-model="suggestionCategory"
          :disabled="busy"
          maxlength="80"
          required
          placeholder="For example, fitness or learning"
          class="rounded-xl bg-white/5 p-3 text-zinc-100"
      /></label>
      <p class="m-0 text-caption text-zinc-500">
        Only this category label is sent to your configured model after it is verified as local. The
        suggestion stays unsaved until you approve it.
      </p>
      <button
        type="submit"
        :disabled="busy || !suggestionCategory.trim()"
        class="self-start cursor-pointer rounded-full border border-white/20 px-4 py-2 text-label text-zinc-200 disabled:opacity-40"
      >
        {{ generating ? 'Drafting style…' : 'Suggest personality' }}
      </button>
    </form>
    <p v-if="suggestion" role="status" class="m-0 text-label text-zinc-300">
      Unsaved suggestion. Review its name, symbol, categories, and communication style before
      approving.
    </p>
    <div class="flex flex-wrap gap-2">
      <button
        v-for="profile in catalog?.entries"
        :key="profile.id"
        :disabled="busy"
        class="cursor-pointer rounded-full border border-white/15 px-4 py-2 text-label text-zinc-200"
        :aria-pressed="draft?.id === profile.id"
        @click="edit(profile)"
      >
        <PersonalitySymbol :mark="profile.mark" class="mr-2 inline-block size-4 align-middle" />{{
          profile.name
        }}
      </button>
      <button
        :disabled="busy"
        class="cursor-pointer rounded-full border border-white/15 px-4 py-2 text-label text-zinc-200"
        @click="create"
      >
        Add personality
      </button>
    </div>
    <form
      v-if="draft"
      aria-label="Edit personality"
      class="flex max-w-2xl flex-col gap-4 rounded-2xl border border-white/10 p-5 text-label"
      @submit.prevent="save"
    >
      <fieldset :disabled="busy" class="m-0 flex flex-col gap-4 border-0 p-0">
        <label class="flex flex-col gap-2"
          >ID<input
            v-model="draft.id"
            required
            pattern="[a-z0-9-]+"
            maxlength="64"
            :disabled="expected !== null"
            class="rounded-xl bg-white/5 p-3 text-zinc-100"
        /></label>
        <div class="flex gap-3">
          <label class="flex flex-1 flex-col gap-2"
            >Name<input
              v-model="draft.name"
              required
              maxlength="80"
              class="min-w-0 rounded-xl bg-white/5 p-3 text-zinc-100"
          /></label>
          <label class="flex w-44 flex-col gap-2"
            >Orb symbol<span class="flex items-center gap-2"
              ><PersonalitySymbol :mark="draft.mark" class="size-5 shrink-0" /><select
                v-model="draft.mark"
                class="min-w-0 rounded-xl bg-[#1b1b1b] p-3 text-zinc-100"
              >
                <option v-for="symbol in personalitySymbols" :key="symbol.id" :value="symbol.id">
                  {{ symbol.label }}
                </option>
              </select></span
            ></label
          >
        </div>
        <label v-if="draft.id !== 'base'" class="flex flex-col gap-2"
          >Categories (comma separated)<input
            v-model="categories"
            placeholder="fitness, running"
            class="rounded-xl bg-white/5 p-3 text-zinc-100"
        /></label>
        <label class="flex flex-col gap-2"
          >Communication style<textarea
            v-model="draft.prompt"
            required
            rows="7"
            maxlength="8000"
            class="resize-y rounded-xl bg-white/5 p-3 text-zinc-100"
          />
        </label>
        <p class="m-0 text-caption text-zinc-400">
          Keep personal records in protected goal conversations. These editable style files contain
          instructions, not your private goal context. Saving explicitly approves this personality
          and its category assignments.
        </p>
        <div class="flex flex-wrap gap-3">
          <button
            type="submit"
            class="cursor-pointer rounded-full bg-zinc-100 px-5 py-2 text-zinc-950"
          >
            {{ pending ? 'Saving…' : 'Approve and save' }}</button
          ><button
            type="button"
            class="cursor-pointer rounded-full border border-white/15 px-5 py-2 text-zinc-200"
            @click="load"
          >
            Reload saved version
          </button>
        </div>
      </fieldset>
    </form>
    <p v-if="error" role="alert" class="m-0 text-label text-[#ff6b72]">
      {{ error }} Your draft is preserved.
    </p>
    <p v-if="saved" role="status" class="m-0 text-label text-zinc-300">
      Personality saved. New model turns use the approved style.
    </p>
  </div>
</template>