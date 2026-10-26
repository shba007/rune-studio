<script setup lang="ts">
import { computed, ref, useId } from 'vue'
import { knowledgeConcepts, knowledgeLinks, type KnowledgeSection } from '@/config/knowledge'
const emit = defineEmits<{ navigate: [section: KnowledgeSection] }>()
const selected = ref('goals')
const concept = computed(() => knowledgeConcepts.find((node) => node.id === selected.value)!)
const lookup = new Map(knowledgeConcepts.map((node) => [node.id, node]))
const connections = computed(() =>
  knowledgeLinks.filter(([from, to]) => from === selected.value || to === selected.value),
)
const neighbors = computed(
  () => new Set<string>(connections.value.flatMap(([from, to]) => [from, to])),
)
const pattern = useId()
const zoom = ref(1)
const offset = ref({ x: 0, y: 0 })
let drag: { id: number; x: number; y: number; startX: number; startY: number } | null = null
function reset() {
  zoom.value = 1
  offset.value = { x: 0, y: 0 }
}
function changeZoom(delta: number) {
  zoom.value = Math.min(1.8, Math.max(0.7, Math.round((zoom.value + delta) * 10) / 10))
}
function startPan(event: PointerEvent) {
  if (event.button !== 0 || !(event.currentTarget instanceof SVGSVGElement)) return
  event.currentTarget.setPointerCapture(event.pointerId)
  drag = {
    id: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    startX: offset.value.x,
    startY: offset.value.y,
  }
}
function pan(event: PointerEvent) {
  if (!drag || drag.id !== event.pointerId || !(event.currentTarget instanceof SVGSVGElement))
    return
  const rect = event.currentTarget.getBoundingClientRect()
  const scale = Math.min(rect.width / 900, rect.height / 700)
  if (scale <= 0) return
  offset.value = {
    x: drag.startX + (event.clientX - drag.x) / scale,
    y: drag.startY + (event.clientY - drag.y) / scale,
  }
}
function keyboardPan(event: KeyboardEvent) {
  if (event.target !== event.currentTarget) return
  const directions: Record<string, [number, number]> = {
    ArrowLeft: [-25, 0],
    ArrowRight: [25, 0],
    ArrowUp: [0, -25],
    ArrowDown: [0, 25],
  }
  const direction = directions[event.key]
  if (direction) {
    event.preventDefault()
    offset.value = { x: offset.value.x + direction[0], y: offset.value.y + direction[1] }
  }
}
</script>
<template>
  <div class="flex flex-col gap-6">
    <section class="rounded-2xl border border-white/10 bg-[#101010] p-5">
      <h2 class="m-0 text-body text-zinc-100">Goals anchor the Life Harness</h2>
      <p class="mb-0 text-label leading-relaxed text-zinc-400">
        Type or say a request. Rune identifies its scope, asks when clarification is needed, and
        keeps the resulting work reviewable. A goal describes where you want to be; a task produces
        a bounded result; a step belongs inside a task. Knowledge brings those conversations,
        results, and styles together. Goals anchor the longer-term loop and connect directly to
        approved personalities. You can still run standalone tasks and steps.
      </p>
      <div class="mt-4 grid gap-3 text-label sm:grid-cols-3">
        <div class="rounded-xl bg-white/5 p-3">
          <span class="block text-caption text-zinc-500">01 · DEFINE</span>
          <p class="mb-0 text-zinc-200">“I want to be physically fit.”</p>
          <p class="mb-0 text-zinc-400">Clarify your starting point and approve a focused goal.</p>
        </div>
        <div class="rounded-xl bg-white/5 p-3">
          <span class="block text-caption text-zinc-500">02 · DO</span>
          <p class="mb-0 text-zinc-200">“Calculate 14 + 27.”</p>
          <p class="mb-0 text-zinc-400">
            Run a bounded task; its answer appears in the orb bubble.
          </p>
        </div>
        <div class="rounded-xl bg-white/5 p-3">
          <span class="block text-caption text-zinc-500">03 · REVIEW</span>
          <p class="mb-0 text-zinc-200">“What happened?”</p>
          <p class="mb-0 text-zinc-400">Open Tasks for results, attempts, captures, and replay.</p>
        </div>
      </div>
    </section>
    <section
      aria-label="How Rune connects"
      class="overflow-hidden rounded-2xl border border-white/10 bg-[#0b0b0b]"
    >
      <header
        class="flex flex-wrap items-center justify-between gap-3 border-b border-white/8 px-5 py-4"
      >
        <div>
          <h2 class="m-0 text-body text-zinc-100">How everything connects</h2>
          <p class="mb-0 text-caption text-zinc-500">
            Concept map · select a node to explore · drag the background to pan
          </p>
        </div>
        <div class="flex items-center gap-1 text-label text-zinc-300">
          <button
            aria-label="Zoom out"
            :disabled="zoom <= 0.7"
            class="grid size-8 cursor-pointer place-items-center rounded-full hover:bg-white/10 disabled:opacity-30"
            @click="changeZoom(-0.1)"
          >
            −
          </button>
          <span class="min-w-12 text-center text-caption tabular-nums"
            >{{ Math.round(zoom * 100) }}%</span
          >
          <button
            aria-label="Zoom in"
            :disabled="zoom >= 1.8"
            class="grid size-8 cursor-pointer place-items-center rounded-full hover:bg-white/10 disabled:opacity-30"
            @click="changeZoom(0.1)"
          >
            +
          </button>
          <button
            class="ml-2 cursor-pointer rounded-full border border-white/15 px-3 py-1 text-caption hover:bg-white/5"
            @click="reset"
          >
            Reset view
          </button>
        </div>
      </header>
      <svg
        viewBox="0 0 900 700"
        class="block max-h-[620px] w-full touch-none outline-none focus-visible:ring-1 focus-visible:ring-white/40"
        tabindex="0"
        role="group"
        aria-label="Rune concept graph. Use arrow keys to pan, or tab to a concept and press Enter."
        @pointerdown="startPan"
        @pointermove="pan"
        @pointerup="drag = null"
        @pointercancel="drag = null"
        @lostpointercapture="drag = null"
        @keydown="keyboardPan"
      >
        <defs>
          <pattern :id="pattern" width="24" height="24" patternUnits="userSpaceOnUse">
            <circle cx="1" cy="1" r="0.7" fill="white" opacity="0.08" />
          </pattern>
        </defs>
        <rect width="900" height="700" :fill="`url(#${pattern})`" />
        <g
          :transform="`translate(${450 + offset.x} ${350 + offset.y}) scale(${zoom}) translate(-450 -350)`"
        >
          <line
            v-for="[from, to, relation] in knowledgeLinks"
            :key="`${from}-${to}`"
            :x1="lookup.get(from)!.x"
            :y1="lookup.get(from)!.y"
            :x2="lookup.get(to)!.x"
            :y2="lookup.get(to)!.y"
            :stroke="from === selected || to === selected ? '#a1a1aa' : '#303034'"
            :stroke-width="from === selected || to === selected ? 1.5 : 1"
            :stroke-dasharray="relation.startsWith('planned:') ? '5 6' : undefined"
          >
            <title>
              {{ lookup.get(from)!.title }} → {{ relation }} → {{ lookup.get(to)!.title }}
            </title>
          </line>
          <g
            v-for="node in knowledgeConcepts"
            :key="node.id"
            role="button"
            tabindex="0"
            :aria-label="`${node.title}: ${node.status}`"
            :aria-pressed="selected === node.id"
            class="concept-node cursor-pointer outline-none"
            :transform="`translate(${node.x} ${node.y})`"
            @pointerdown.stop
            @click="selected = node.id"
            @keydown.enter.prevent="selected = node.id"
            @keydown.space.prevent="selected = node.id"
          >
            <circle r="28" fill="transparent" />
            <circle
              :r="node.id === 'goals' ? 18 : 11"
              :fill="selected === node.id ? '#e4e4e7' : '#151515'"
              :stroke="
                selected === node.id ? '#fafafa' : neighbors.has(node.id) ? '#a1a1aa' : '#52525b'
              "
              :stroke-width="selected === node.id ? 2 : 1.5"
              :stroke-dasharray="node.status === 'Planned' ? '3 3' : undefined"
            />
            <text y="43" text-anchor="middle" fill="#e4e4e7" font-size="13" class="select-none">
              {{ node.title }}
            </text>
            <text
              v-if="node.status === 'Planned' || node.status === 'Partial'"
              y="60"
              text-anchor="middle"
              fill="#71717a"
              font-size="10"
              class="select-none"
            >
              {{ node.status }}
            </text>
          </g>
        </g>
      </svg>
      <p class="m-0 border-t border-white/8 px-5 py-3 text-caption text-zinc-500">
        Solid connections describe current relationships. Dashed connections are planned. This is a
        product guide, not a graph of your private data.
      </p>
    </section>
    <section aria-label="Concept explanation" class="rounded-2xl border border-white/10 p-5">
      <label class="flex flex-col gap-2 text-label text-zinc-400"
        >Explore every concept<select
          v-model="selected"
          class="max-w-sm rounded-xl bg-[#1b1b1b] px-3 py-2 text-zinc-100"
        >
          <option v-for="node in knowledgeConcepts" :key="node.id" :value="node.id">
            {{ node.title }} · {{ node.status }}
          </option>
        </select></label
      >
      <div class="mt-5 flex flex-wrap items-center gap-3">
        <h2 class="m-0 text-title text-zinc-100">{{ concept.title }}</h2>
        <span class="rounded-full border border-white/15 px-3 py-1 text-caption text-zinc-400">{{
          concept.status
        }}</span>
      </div>
      <p class="text-body leading-relaxed text-zinc-300">{{ concept.meaning }}</p>
      <p class="rounded-xl bg-white/5 p-4 text-label leading-relaxed text-zinc-300">
        <span class="mb-1 block text-caption text-zinc-500 uppercase">Example</span
        >{{ concept.example }}
      </p>
      <h3 class="text-label font-normal text-zinc-100">What works today, and what does not</h3>
      <p class="text-label leading-relaxed text-zinc-400">{{ concept.today }}</p>
      <button
        v-if="concept.section && concept.section !== 'overview'"
        class="cursor-pointer rounded-full border border-white/20 px-4 py-2 text-label text-zinc-100 hover:bg-white/5"
        @click="emit('navigate', concept.section)"
      >
        Open
        {{
          concept.section === 'personalities'
            ? 'Personalities'
            : concept.section === 'goals'
              ? 'Goals'
              : 'Tasks'
        }}
      </button>
      <div v-if="connections.length" class="mt-5 border-t border-white/10 pt-4">
        <h3 class="m-0 text-label font-normal text-zinc-300">Connected concepts</h3>
        <ul class="mb-0 flex list-none flex-col gap-2 p-0 text-label text-zinc-400">
          <li v-for="[from, to, relation] in connections" :key="`${from}-${to}`">
            <button
              class="cursor-pointer text-left hover:text-white"
              @click="selected = from === selected ? to : from"
            >
              {{ lookup.get(from)!.title }} <span class="text-zinc-600">→ {{ relation }} →</span>
              {{ lookup.get(to)!.title }}
            </button>
          </li>
        </ul>
      </div>
    </section>
    <section class="grid gap-4 sm:grid-cols-2">
      <div class="rounded-2xl border border-white/10 p-5">
        <h2 class="m-0 text-body text-zinc-100">You stay in control</h2>
        <p class="mb-0 text-label leading-relaxed text-zinc-400">
          You approve goal definitions and personality edits. Capture and recording have explicit
          controls. Tools keep their permissions. Private goal storage requires consent; other
          archives have their own controls. A personality changes communication style, not what Rune
          is allowed to do.
        </p>
      </div>
      <div class="rounded-2xl border border-dashed border-white/15 p-5">
        <h2 class="m-0 text-body text-zinc-100">The Life Harness direction</h2>
        <p class="mb-0 text-label leading-relaxed text-zinc-400">
          The intended loop is clarify → compare pathways → agree on work → act → review evidence →
          adapt. Goal clarification and pathway comparison are available; automatic links from a
          selected route to execution are future work. Pathways may span a week, a month, or a year.
          Explicit task reminders, parent links and user-reported outcome tracking are available.
          Proactive coaching, broad context collection and automatic outcome measurement remain
          future cycles.
        </p>
      </div>
    </section>
  </div>
</template>
<style scoped>
.concept-node:focus-visible > circle:nth-child(2) {
  stroke: white;
  stroke-width: 3;
}
.concept-node:hover > circle:nth-child(2) {
  stroke: white;
}
</style>