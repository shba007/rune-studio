export interface ReminderPlan {
  title: string
  local: string
  due: Date
}

const WORDS: Record<string, number> = {
  a: 1,
  an: 1,
  one: 1,
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
  nine: 9,
  ten: 10,
  fifteen: 15,
  twenty: 20,
  thirty: 30,
}
const NUMBER = `\\d+(?:\\.\\d+)?|${Object.keys(WORDS).join('|')}`
const RELATIVE = `(?:in|after)\\s+(?:half\\s+an?\\s+(hour)|(${NUMBER})\\s*(seconds?|secs?|s|minutes?|mins?|m|hours?|hrs?|h|days?|d)\\b)`
const ABSOLUTE = `(?:(tomorrow|today|tonight)\\s+)?at\\s+(\\d{1,2})(?::(\\d{2}))?\\s*(am|pm|a\\.m\\.|p\\.m\\.)?`
const DAY_ONLY = `tomorrow(?:\\s+morning)?`
const TIME = `(?:${RELATIVE}|${ABSOLUTE}|(${DAY_ONLY}))`
const LEADING = new RegExp(`^${TIME}(?=\\s|$)\\s*(.*)$`, 'i')
const TRAILING = new RegExp(`^(.*?)\\s*[,;]?\\s+${TIME}\\s*[.!]?$`, 'i')
const CONNECTOR = /^(?:to|that|about|of|:|,|-)\s+/i
const SUBJECT =
  /^(?:i|we)(?:'ve| have| need| should| must| want| gotta| got)?\s*(?:to|got to)\s+|^me\s+to\s+/i

const pad = (n: number) => String(n).padStart(2, '0')
export const localStamp = (d: Date) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`

function ceilToMinute(ms: number) {
  return Math.ceil(ms / 60000) * 60000
}

function dueFrom(match: RegExpMatchArray, offset: number, now: Date): Date | string {
  const g = (i: number) => match[offset + i]
  if (g(0) !== undefined) return new Date(ceilToMinute(now.getTime() + 30 * 60000))
  if (g(2) !== undefined) {
    const raw = g(1)!.toLowerCase()
    const count = WORDS[raw] ?? Number(raw)
    const unit = g(2)!.toLowerCase()
    const minutes = unit.startsWith('s')
      ? count / 60
      : unit.startsWith('m')
        ? count
        : unit.startsWith('h')
          ? count * 60
          : count * 1440
    if (!(minutes > 0) || minutes > 60 * 24 * 60) return 'Choose a reminder time within 60 days.'
    return new Date(ceilToMinute(now.getTime() + minutes * 60000))
  }
  if (g(4) !== undefined) {
    let hour = Number(g(4))
    const minute = g(5) === undefined ? 0 : Number(g(5))
    const meridiem = g(6)?.replace(/\./g, '').toLowerCase()
    if (meridiem && (hour < 1 || hour > 12)) return 'That time is not valid.'
    if (meridiem === 'pm' && hour < 12) hour += 12
    if (meridiem === 'am' && hour === 12) hour = 0
    if (hour > 23 || minute > 59) return 'That time is not valid.'
    const day = g(3)?.toLowerCase()
    const due = new Date(now)
    due.setHours(hour, minute, 0, 0)
    if (day === 'tomorrow') due.setDate(due.getDate() + 1)
    else if (due.getTime() <= now.getTime()) {
      if (day === 'today') return 'That time has already passed today.'
      due.setDate(due.getDate() + 1)
    }
    return due
  }
  const due = new Date(now)
  due.setDate(due.getDate() + 1)
  due.setHours(9, 0, 0, 0)
  return due
}

function titleOf(raw: string) {
  let text = raw.trim().replace(/^[,;:\s]+|[\s.!,;]+$/g, '')
  text = text.replace(CONNECTOR, '').replace(SUBJECT, '').trim()
  if (!text) return ''
  return text.charAt(0).toUpperCase() + text.slice(1)
}

function interpret(rest: string, now: Date): ReminderPlan | string {
  const text = rest.trim()
  const lead = text.match(LEADING)
  const trail = lead ? null : text.match(TRAILING)
  if (!lead && !trail) return 'Say when, for example: in 10 minutes, at 5pm, or tomorrow.'
  const due = lead ? dueFrom(lead, 1, now) : dueFrom(trail!, 2, now)
  if (typeof due === 'string') return due
  const title = titleOf(lead ? lead[lead.length - 1]! : trail![1]!)
  if (!title) return 'Say what to remind you about.'
  return { title, due, local: localStamp(due) }
}

export function parseReminder(text: string, now = new Date()): ReminderPlan | null {
  const match = text.trim().match(/^remind(?:\s+me)?\s+(.+)$/i)
  if (!match) return null
  const plan = interpret(match[1]!, now)
  return typeof plan === 'string' ? null : plan
}

export function parseSchedule(text: string, now = new Date()): ReminderPlan | string {
  const rest = text.trim().replace(/^remind(?:\s+me)?\s+/i, '')
  if (!rest) return 'Tell Rune when and what, for example: /schedule in 10 minutes drink water'
  return interpret(rest, now)
}
export interface Interpretation {
  kind: 'after' | 'at' | 'unclear'
  title: string
  delaySeconds: number
  local: string
}

export function planFromInterpretation(
  result: Interpretation,
  now = new Date(),
): ReminderPlan | null {
  if (result.kind === 'unclear' || !result.title.trim()) return null
  if (result.kind === 'after') {
    const due = new Date(ceilToMinute(now.getTime() + result.delaySeconds * 1000))
    return { title: result.title.trim(), due, local: localStamp(due) }
  }
  const m = result.local.match(/^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/)
  if (!m) return null
  const due = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]))
  if (Number.isNaN(due.getTime()) || due.getTime() <= now.getTime()) return null
  return { title: result.title.trim(), due, local: localStamp(due) }
}