import { expect, it } from 'vite-plus/test'

import { pillCards, type PillCardState } from './pillCards'

const idle: PillCardState = {
  question: false,
  analysis: false,
  decision: false,
  decisionInKnowledge: false,
  taskPanel: false,
  board: false,
  working: false,
  server: false,
  reflection: false,
  review: false,
  feedback: false,
  update: false,
}
const show = (patch: Partial<PillCardState>) => pillCards({ ...idle, ...patch })

it('shows only the confirm card while a question is open', () => {
  expect(
    show({
      question: true,
      analysis: true,
      decision: true,
      board: true,
      update: true,
      review: true,
    }),
  ).toEqual({ cards: ['confirm'], more: 0 })
})

it('picks one primary card in order analysis, decision, task panel, board', () => {
  expect(show({ analysis: true, decision: true, taskPanel: true, board: true }).cards).toEqual([
    'analysis',
  ])
  expect(show({ decision: true, taskPanel: true, board: true }).cards).toEqual(['decision'])
  expect(show({ decisionInKnowledge: true, decision: true }).cards).toEqual(['decisionLink'])
  expect(show({ taskPanel: true, board: true }).cards).toEqual(['taskPanel', 'reminder'])
  expect(show({ board: true }).cards).toEqual(['board', 'reminder'])
})

it('shows one quiet card in order and counts the rest', () => {
  expect(show({ reflection: true, review: true, update: true })).toEqual({
    cards: ['reflection'],
    more: 2,
  })
  expect(show({ review: true, feedback: true, update: true })).toEqual({
    cards: ['review'],
    more: 1,
  })
  expect(show({ feedback: true })).toEqual({ cards: ['feedback'], more: 0 })
  expect(show({ update: true })).toEqual({ cards: ['update'], more: 0 })
})

it('keeps quiet cards away while work, analysis or a decision is active', () => {
  expect(show({ board: true, working: true, update: true })).toEqual({
    cards: ['board'],
    more: 0,
  })
  expect(show({ decision: true, update: true }).cards).toEqual(['decision'])
  expect(show({ analysis: true, review: true }).cards).toEqual(['analysis'])
})

it('puts the server card ahead of the other quiet cards', () => {
  expect(show({ server: true, reflection: true, update: true })).toEqual({
    cards: ['server'],
    more: 2,
  })
  expect(show({ server: true, working: true }).cards).toEqual([])
  expect(show({ server: true, question: true }).cards).toEqual(['confirm'])
})

it('keeps a finished board beside a quiet card', () => {
  expect(show({ board: true, review: true })).toEqual({ cards: ['board', 'review'], more: 0 })
})

it('falls back to the reminder only when nothing quiet is waiting', () => {
  expect(show({})).toEqual({ cards: ['reminder'], more: 0 })
})