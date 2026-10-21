import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { lookFor } from '@/config/agents'
import { EXTERNAL_AGENT, type ExternalAgentPayload } from '@/types/protocol'

/** A payload where nobody is in control is "no identity". */
export function readIdentity(raw: unknown): ExternalAgentPayload | null {
  if (typeof raw !== 'object' || raw === null) return null
  const value = raw as Record<string, unknown>
  if (typeof value.agent !== 'string' || value.agent === '') return null
  return {
    agent: value.agent,
    id: typeof value.id === 'string' ? value.id : 'external',
    known: value.known === true,
    verified: value.verified === true,
    declared: typeof value.declared === 'string' ? value.declared : null,
  }
}

/**
 * Which outside agent is in control, for the orb, its ring and the top bar. `null` while Rune's own
 * agent (or nobody) is. The label is a courtesy: the agent declared it (see `bridge::identity`).
 */
export function useExternalAgent() {
  const identity = ref<ExternalAgentPayload | null>(null)
  let unlisten: UnlistenFn | undefined

  const look = computed(() =>
    identity.value ? lookFor(identity.value.id, identity.value.agent) : null,
  )
  const orbAccent = computed(() => look.value?.accent ?? null)
  const darkAccent = computed(() => look.value?.accentOnDark ?? null)
  const mark = computed(() => look.value?.glyph ?? null)
  const logo = computed(() => look.value?.logo ?? null)
  const mono = computed(() => look.value?.mono ?? false)
  const letter = computed(() => look.value?.letter ?? null)
  /** "Declared by the agent" unless the registration attested it. */
  const hint = computed(() => {
    const who = identity.value
    if (!who) return ''
    if (!who.verified && who.known) return 'Declared by the agent'
    if (!who.known && who.declared) return `Declared by the agent: ${who.declared}`
    return ''
  })

  onMounted(async () => {
    unlisten = await listen<ExternalAgentPayload>(EXTERNAL_AGENT, (event) => {
      identity.value = readIdentity(event.payload)
    })
    // A window that opens while an agent is in control asks for it.
    try {
      identity.value = readIdentity(await invoke('external_agent'))
    } catch {
      identity.value = null
    }
  })
  onBeforeUnmount(() => unlisten?.())

  return { identity, orbAccent, darkAccent, mark, logo, mono, letter, hint }
}