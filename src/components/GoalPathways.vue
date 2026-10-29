<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import type { IntakeEntry } from '@/types/decisions'
import AppIcon from './AppIcon.vue'
defineProps<{ entry: IntakeEntry; busy: boolean }>()
defineEmits<{ select: [id: string] }>()
</script>
<template>
  <section
    v-if="entry.pathways"
    aria-label="Goal pathways"
    class="flex flex-col gap-rune-3 pt-6 text-rune-body"
  >
    <header class="flex flex-wrap items-end justify-between gap-rune-2">
      <div>
        <p class="m-0 mb-1 text-rune-caption uppercase tracking-wider text-rune-text-2">
          Choose your approach
        </p>
        <h3 class="m-0 font-sans text-rune-subtitle font-medium text-rune-ink">
          {{
            entry.pathways.options.length
              ? `${entry.pathways.options.length} pathways to consider`
              : 'More context needed'
          }}
        </h3>
      </div>
      <span class="text-rune-caption text-rune-text-2"
        >Ranked by fit · estimates, not guarantees</span
      >
    </header>
    <div v-if="entry.pathways.approved" role="status" class="flex items-start gap-rune-2 py-1 pl-4">
      <AppIcon name="ok" class="mt-1 shrink-0 text-rune-ink" />
      <div>
        <p class="m-0 text-rune-caption text-rune-text-2">Your preferred route</p>
        <p class="m-0 mt-1 text-rune-body text-rune-ink">{{ entry.pathways.approved.title }}</p>
        <p class="m-0 mt-1 text-rune-caption text-rune-text-2">Saved as a preference only.</p>
      </div>
    </div>
    <details
      class="text-rune-body text-rune-text-2"
      :open="
        !entry.pathways.options.some((route) =>
          ['feasible', 'conditional'].includes(route.feasibility),
        )
      "
    >
      <summary class="w-fit cursor-pointer py-1 hover:text-rune-ink">
        Read the feasibility assessment
      </summary>
      <p class="mb-0 max-w-3xl leading-relaxed">{{ entry.pathways.assessment }}</p>
      <p v-if="entry.pathways.similarOutcome" class="mb-0 max-w-3xl leading-relaxed">
        <span class="text-rune-ink">Alternative outcome to clarify:</span>
        {{ entry.pathways.similarOutcome }}
      </p>
    </details>
    <p v-if="!entry.pathways.options.length" class="m-0 max-w-2xl leading-relaxed text-rune-text-2">
      No supported route was proposed. Refine the goal or provide the missing context before
      comparing again.
    </p>
    <div class="grid items-stretch gap-4 lg:grid-cols-3">
      <article
        v-for="(route, index) in entry.pathways.options"
        :key="route.id"
        class="flex min-w-0 flex-col rounded-rune-tile p-rune-3"
        :class="entry.pathways.selected === route.id ? 'bg-white/[0.045]' : 'bg-rune-sunken'"
      >
        <header class="mb-4 flex items-center justify-between gap-2">
          <span class="text-[28px] leading-none font-light tabular-nums text-rune-text-3">{{
            String(index + 1).padStart(2, '0')
          }}</span
          ><span
            v-if="entry.pathways.selected === route.id"
            class="inline-flex items-center gap-1 text-rune-caption text-rune-ink"
            ><AppIcon name="ok" size="sm" />Preferred</span
          ><span
            v-else-if="index === 0 && ['feasible', 'conditional'].includes(route.feasibility)"
            class="rounded-full bg-rune-surface px-2.5 py-1 text-rune-caption text-rune-ink"
            >Recommended</span
          >
        </header>
        <h4 class="m-0 text-rune-body leading-relaxed font-medium text-rune-ink">
          {{ route.title }}
        </h4>
        <p class="mt-2 mb-rune-3 line-clamp-3 leading-relaxed text-rune-text-2">
          {{ route.summary }}
        </p>
        <dl class="m-0 flex flex-col gap-rune-2 py-4 text-rune-caption">
          <div class="flex items-start gap-2.5">
            <AppIcon name="wait" size="sm" class="mt-0.5 shrink-0 text-rune-text-2" />
            <div class="min-w-0">
              <dt class="text-rune-text-2">Estimated duration</dt>
              <dd class="m-0 mt-1 line-clamp-2 text-rune-ink">{{ route.duration }}</dd>
            </div>
          </div>
          <div class="flex items-center gap-2.5">
            <AppIcon
              :name="route.feasibility === 'feasible' ? 'ok' : 'shield'"
              size="sm"
              class="shrink-0 text-rune-text-2"
            />
            <div>
              <dt class="sr-only">Feasibility</dt>
              <dd class="m-0 capitalize text-rune-text-2">{{ route.feasibility }}</dd>
            </div>
          </div>
        </dl>
        <div class="flex flex-col gap-rune-2 py-4 text-rune-caption leading-relaxed">
          <div v-if="route.pros.length" class="flex gap-2.5">
            <span class="text-rune-text-2" aria-hidden="true">+</span>
            <p class="m-0 line-clamp-2 text-rune-text-2">{{ route.pros[0] }}</p>
          </div>
          <div v-if="route.cons.length" class="flex gap-2.5">
            <span class="text-rune-text-2" aria-hidden="true">−</span>
            <p class="m-0 line-clamp-2 text-rune-text-2">{{ route.cons[0] }}</p>
          </div>
        </div>
        <details class="mb-rune-3 text-rune-caption text-rune-text-2">
          <summary class="cursor-pointer py-1 hover:text-rune-ink">
            {{
              route.missingInformation.length
                ? `${route.missingInformation.length} open question${route.missingInformation.length === 1 ? '' : 's'} · `
                : ''
            }}Route details
          </summary>
          <p class="leading-relaxed">{{ route.summary }}</p>
          <p class="leading-relaxed">
            <span class="text-rune-ink">Why this ranking:</span> {{ route.rationale }}
          </p>
          <p><span class="text-rune-text-2">Duration:</span> {{ route.duration }}</p>
          <dl class="m-0 flex flex-col gap-rune-2">
            <div
              v-for="field in ['pros', 'cons', 'resources', 'missingInformation'] as const"
              :key="field"
            >
              <dt class="mb-1 text-rune-ink">
                {{
                  {
                    pros: 'Pros',
                    cons: 'Cons',
                    resources: 'Resources needed',
                    missingInformation: 'Missing information',
                  }[field]
                }}
              </dt>
              <dd class="m-0">
                <ul v-if="route[field].length" class="m-0 list-disc space-y-1 pl-4 leading-relaxed">
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
            !['feasible', 'conditional'].includes(route.feasibility) ||
            entry.pathways.selected === route.id
          "
          @click="$emit('select', route.id)"
        >
          {{
            entry.pathways.selected === route.id
              ? 'Preferred route saved'
              : !['feasible', 'conditional'].includes(route.feasibility)
                ? 'Clarify feasibility first'
                : 'Save as preferred route'
          }}
        </RuneButton>
      </article>
    </div>
    <p class="m-0 flex items-start gap-2 text-rune-caption leading-relaxed text-rune-text-2">
      <AppIcon name="shield" size="sm" class="mt-0.5 shrink-0" />Saving a preferred route does not
      create tasks, reminders, schedules, or authorize execution.
    </p>
  </section>
</template>