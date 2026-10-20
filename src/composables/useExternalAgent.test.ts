import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'

import AgentMark from '@/components/AgentMark.vue'
import { AGENT_BRANDS } from '@/config/agents'

import { readIdentity } from './useExternalAgent'

describe('readIdentity', () => {
  it('reads a known, verified agent', () => {
    expect(
      readIdentity({
        agent: 'Claude',
        id: 'claude',
        known: true,
        verified: true,
        declared: 'claude-code',
      }),
    ).toEqual({
      agent: 'Claude',
      id: 'claude',
      known: true,
      verified: true,
      declared: 'claude-code',
    })
  })

  it('reads nobody in control as no identity, and garbage as none too', () => {
    expect(
      readIdentity({ agent: null, id: null, known: false, verified: false, declared: null }),
    ).toBeNull()
    expect(readIdentity(null)).toBeNull()
    expect(readIdentity('Claude')).toBeNull()
  })

  it('gives an agent without an id the neutral one', () => {
    expect(readIdentity({ agent: 'Acme Bot' })?.id).toBe('external')
  })
})

describe('AgentMark', () => {
  it('draws the brand glyph as one filled path in the current colour', () => {
    const wrapper = mount(AgentMark, { props: { glyph: AGENT_BRANDS.claude!.glyph } })
    expect(wrapper.find('svg path').attributes('d')).toBe(AGENT_BRANDS.claude!.glyph)
    expect(wrapper.find('svg').attributes('fill')).toBe('currentColor')
  })

  it("draws a neutral icon for an agent with no glyph, and Rune's own mark when asked", () => {
    const neutral = mount(AgentMark, { props: { glyph: null } })
    const own = mount(AgentMark, { props: { glyph: null, own: true } })
    expect(neutral.html()).not.toContain(AGENT_BRANDS.claude!.glyph ?? 'x')
    expect(neutral.find('svg').exists()).toBe(true)
    expect(own.find('svg').exists()).toBe(true)
    expect(neutral.html()).not.toBe(own.html())
  })
})