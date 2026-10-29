<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import type { IntakeEntry } from '@/types/decisions'
import AppIcon from './AppIcon.vue'
import RuneChip from './RuneChip.vue'
defineProps<{ entry: IntakeEntry; busy: boolean }>()
defineEmits<{ select: [id: string] }>()
const usable = (feasibility: string) => ['feasible', 'conditional'].includes(feasibility)
const detailLabels = {
  pros: 'Pros',
  cons: 'Cons',
  resources: 'Resources needed',
  missingInformation: 'Missing information',
} as const
</script>
<template>
  <section
    v-if="entry.pathways"
    aria-label="Goal pathways"
    class="flex flex-col gap-rune-2 text-rune-body"
  >
    <header class="flex flex-wrap items-end justify-between gap-rune-2 px-rune-2">
      <div class="flex flex-col gap-1">
        <h3 class="m-0 font-sans text-rune-heading text-rune-text-3 uppercase">
          Choose your approach
        </h3>
        <p class="m-0 font-sans text-rune-subtitle text-rune-ink">
          {{
            entry.pathways.options.length
              ? `${entry.pathways.options.length} pathways to consider`
              : 'More context needed'
          }}
        </p>
      </div>
      <span class="text-rune-caption text-rune-text-3"
        >Ranked by fit · estimates, not guarantees</span
      >
    </header>
    <div
      v-if="entry.pathways.approved"
      role="status"
      class="flex items-start gap-rune-2 rounded-rune-tile bg-rune-surface p-rune-2"
    >
      <AppIcon name="ok" class="mt-0.5 shrink-0 text-rune-ink" />
      <div class="flex flex-col gap-1">
        <p class="m-0 text-rune-caption text-rune-text-3">Your preferred route</p>
        <p class="m-0 text-rune-body text-rune-ink">{{ entry.pathways.approved.title }}</p>
        <p class="m-0 text-rune-caption text-rune-text-3">Saved as a preference only.</p>
      </div>
    </div>
    <details
      class="text-rune-body text-rune-text-2"
      :open="!entry.pathways.options.some((route) => usable(route.feasibility))"
    >
      <summary class="w-fit cursor-pointer px-rune-2 py-1 hover:text-rune-ink">
        Read the feasibility assessment
      </summary>
      <p class="mx-rune-2 mt-rune-1 mb-0 max-w-3xl">{{ entry.pathways.assessment }}</p>
      <p v-if="entry.pathways.similarOutcome" class="mx-rune-2 mt-rune-1 mb-0 max-w-3xl">
        <span class="text-rune-ink">Alternative outcome to clarify:</span>
        {{ entry.pathways.similarOutcome }}
      </p>
    </details>
    <p v-if="!entry.pathways.options.length" class="m-0 max-w-2xl px-rune-2 text-rune-text-2">
      No supported route was proposed. Refine the goal or provide the missing context before
      comparing again.
    </p>
    <div
      class="grid items-stretch gap-rune-1"
      :class="entry.pathways.options.length === 2 ? 'lg:grid-cols-2' : 'lg:grid-cols-3'"
    >
      <article
        v-for="(route, index) in entry.pathways.options"
        :key="route.id"
        class="flex min-w-0 flex-col gap-rune-2 rounded-rune-tile p-rune-2"
        :class="entry.pathways.selected === route.id ? 'bg-rune-raised' : 'bg-rune-surface'"
      >
        <header class="flex items-center justify-between gap-2">
          <span class="font-sans text-rune-subtitle tabular-nums text-rune-text-3">{{
            String(index + 1).padStart(2, '0')
          }}</span>
          <RuneChip v-if="entry.pathways.selected === route.id" variant="ink">Preferred</RuneChip>
          <RuneChip v-else-if="index === 0 && usable(route.feasibility)">Recommended</RuneChip>
        </header>
        <div class="flex flex-col gap-1">
          <h4 class="m-0 text-rune-body font-medium text-rune-ink">{{ route.title }}</h4>
          <p class="m-0 line-clamp-3 text-rune-caption text-rune-text-2">{{ route.summary }}</p>
        </div>
        <dl class="m-0 flex flex-col gap-rune-1 text-rune-caption">
          <div class="flex items-start gap-2">
            <AppIcon name="wait" size="sm" class="mt-0.5 shrink-0 text-rune-text-3" />
            <div class="min-w-0">
              <dt class="text-rune-text-3">Estimated duration</dt>
              <dd class="m-0 line-clamp-2 text-rune-ink">{{ route.duration }}</dd>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <AppIcon
              :name="route.feasibility === 'feasible' ? 'ok' : 'shield'"
              size="sm"
              class="shrink-0 text-rune-text-3"
            />
            <div>
              <dt class="sr-only">Feasibility</dt>
              <dd class="m-0 capitalize text-rune-text-2">{{ route.feasibility }}</dd>
            </div>
          </div>
        </dl>
        <div class="flex flex-col gap-rune-1 text-rune-caption">
          <div v-if="route.pros.length" class="flex gap-2">
            <span class="text-rune-text-3" aria-hidden="true">+</span>
            <p class="m-0 line-clamp-2 text-rune-text-2">{{ route.pros[0] }}</p>
          </div>
          <div v-if="route.cons.length" class="flex gap-2">
            <span class="text-rune-text-3" aria-hidden="true">−</span>
            <p class="m-0 line-clamp-2 text-rune-text-2">{{ route.cons[0] }}</p>
          </div>
        </div>
        <details class="text-rune-caption text-rune-text-2">
          <summary class="cursor-pointer py-1 hover:text-rune-ink">
            {{
              route.missingInformation.length
                ? `${route.missingInformation.length} open question${route.missingInformation.length === 1 ? '' : 's'} · `
                : ''
            }}Route details
          </summary>
          <p class="mt-rune-1 mb-0">{{ route.summary }}</p>
          <p class="mt-rune-1 mb-0">
            <span class="text-rune-ink">Why this ranking:</span> {{ route.rationale }}
          </p>
          <p class="mt-rune-1 mb-0">
            <span class="text-rune-ink">Duration:</span> {{ route.duration }}
          </p>
          <dl class="m-0 mt-rune-1 flex flex-col gap-rune-1">
            <div v-for="(label, field) in detailLabels" :key="field">
              <dt class="mb-1 text-rune-ink">{{ label }}</dt>
              <dd class="m-0">
                <ul v-if="route[field].length" class="m-0 list-disc space-y-1 pl-4">
                  <li v-for="item in route[field]" :key="item">{{ item }}</li>
                </ul>
                <span v-else>None stated by the model</span>
              </dd>
            </div>
          </dl>
        </details>
        <RuneButton
          class="mt-auto w-full"
          :variant="index === 0 && entry.pathways.selected !== route.id ? 'primary' : 'secondary'"
          :disabled="
            busy ||
            entry.status !== 'goal_ready' ||
            !usable(route.feasibility) ||
            entry.pathways.selected === route.id
          "
          @click="$emit('select', route.id)"
        >
          {{
            entry.pathways.selected === route.id
              ? 'Preferred route saved'
              : !usable(route.feasibility)
                ? 'Clarify feasibility first'
                : 'Save as preferred route'
          }}
        </RuneButton>
      </article>
    </div>
    <p class="m-0 flex items-start gap-2 px-rune-2 text-rune-caption text-rune-text-3">
      <AppIcon name="shield" size="sm" class="mt-0.5 shrink-0" />Saving a preferred route does not
      create tasks, reminders, schedules, or authorize execution.
    </p>
  </section>
</template>