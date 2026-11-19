import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'

import { closeFocusForm, openFocusForm, useFocus } from '@/composables/useFocus'
import {
  EMPTY_FOCUS,
  type FocusBlock,
  type FocusCommitment,
  type FocusSnapshot,
} from '@/config/focus'

import FocusCard from './FocusCard.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))
const events = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<
    (name: string, handler: (event: { payload: unknown }) => void) => Promise<() => void>
  >(async (name, handler) => {
    events.handlers.set(name, handler)
    return () => events.handlers.delete(name)
  }),
}))
const fire = (name: string, payload: unknown = null) => events.handlers.get(name)?.({ payload })

const running = (patch: Partial<FocusBlock> = {}): FocusBlock => ({
  id: 1,
  title: 'Write auth tests',
  doneWhen: 'tests pass',
  plannedMs: 25 * 60_000,
  startedMs: Date.now(),
  endsMs: Date.now() + 25 * 60_000,
  finishedMs: null,
  outcome: null,
  taskId: null,
  handedOffMs: null,
  ...patch,
})
const focusing = (block = running(), commitments: FocusCommitment[] = []): FocusSnapshot => ({
  ...EMPTY_FOCUS,
  state: 'focusing',
  block,
  recent: [],
  commitments,
})
const idle = (recent: FocusBlock[] = [], commitments: FocusCommitment[] = []): FocusSnapshot => ({
  ...EMPTY_FOCUS,
  state: 'idle',
  block: null,
  recent,
  commitments,
})

async function mountCard(
  initial: FocusSnapshot,
  reply?: (command: string) => FocusSnapshot,
  attachTo?: HTMLElement,
) {
  api.invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'focus_snapshot') return initial
    return reply?.(command) ?? initial
  })
  let focus!: ReturnType<typeof useFocus>
  const wrapper = mount(
    {
      components: { FocusCard },
      setup() {
        focus = useFocus()
        return { focus }
      },
      template: '<FocusCard :focus="focus" />',
    },
    { attachTo },
  )
  await flushPromises()
  return { wrapper, focus }
}
const button = (wrapper: ReturnType<typeof mount>, text: string) =>
  wrapper.findAll('button').find((item) => item.text() === text)!
const calls = (name: string) => api.invoke.mock.calls.filter(([command]) => command === name)

beforeEach(() => closeFocusForm())

it('shows the countdown, the title and the done-when line while focusing', async () => {
  const { wrapper } = await mountCard(focusing())
  expect(wrapper.find('[role=timer]').text()).toMatch(/^2[45]:\d\d$/)
  expect(wrapper.text()).toContain('Write auth tests')
  expect(wrapper.text()).toContain('Done when: tests pass')
  expect(wrapper.text()).toContain('Focusing')
})

it('says the time is up when the block has run out, and leaves it running', async () => {
  const over = running({ endsMs: Date.now() - 1_000 })
  const { wrapper } = await mountCard(focusing(over))
  expect(wrapper.text()).toContain('Time')
  expect(wrapper.text()).toContain('up')
  expect(wrapper.find('[role=timer]').text()).toBe('0:00')
  expect(button(wrapper, 'Done').exists()).toBe(true)
  expect(calls('focus_finish')).toHaveLength(0)
})

it('finishes the block as done and shows it in the recent list', async () => {
  const logged = running({ id: 3, outcome: 'done', finishedMs: Date.now() })
  const { wrapper, focus } = await mountCard(focusing(), () => idle([logged]))
  await button(wrapper, 'Done').trigger('click')
  await flushPromises()
  expect(focus.formOpen.value).toBe(true)
  expect(wrapper.find('[aria-label="Recent focus blocks"]').text()).toContain('Write auth tests')
  expect(calls('focus_finish')[0]![1]).toEqual({ outcome: 'done' })
  expect(wrapper.text()).toContain('Start focus')
})

it('finishes the block as stopped', async () => {
  const { wrapper } = await mountCard(focusing(), () => idle())
  await button(wrapper, 'Stop').trigger('click')
  await flushPromises()
  expect(calls('focus_finish')[0]![1]).toEqual({ outcome: 'stopped' })
})

it('starts a block with the typed title, done-when and the chosen length', async () => {
  const { wrapper } = await mountCard(idle(), () => focusing())
  openFocusForm()
  await flushPromises()
  const [title, doneWhen] = wrapper.findAll('input')
  await title!.setValue('  Draft the spec  ')
  await doneWhen!.setValue('one page written')
  await button(wrapper, '+').trigger('click')
  await button(wrapper, '+').trigger('click')
  await button(wrapper, '−').trigger('click')
  expect(wrapper.find('[aria-label="Block length"]').text()).toBe('30 min')
  await wrapper.find('form').trigger('submit')
  await flushPromises()
  expect(calls('focus_start')[0]![1]).toEqual({
    title: '  Draft the spec  ',
    doneWhen: 'one page written',
    minutes: 30,
  })
  expect(wrapper.find('[role=timer]').exists()).toBe(true)
})

it('does not start without a title', async () => {
  const { wrapper } = await mountCard(idle())
  expect(button(wrapper, 'Start focus').attributes('disabled')).toBeDefined()
  await wrapper.find('form').trigger('submit')
  await flushPromises()
  expect(calls('focus_start')).toHaveLength(0)
})

it('shows the reason when a block cannot start and keeps what was typed', async () => {
  const { wrapper } = await mountCard(idle())
  api.invoke.mockImplementation(async (command) => {
    if (command === 'focus_start') throw 'A focus block is already running. Finish it first.'
    return idle()
  })
  await wrapper.find('input').setValue('Draft')
  await wrapper.find('form').trigger('submit')
  await flushPromises()
  expect(wrapper.find('[role=alert]').text()).toContain('already running')
  expect((wrapper.find('input').element as HTMLInputElement).value).toBe('Draft')
})

it('lists the last finished blocks when idle', async () => {
  const done = running({
    id: 2,
    outcome: 'done',
    startedMs: 0,
    finishedMs: 12 * 60_000,
    endsMs: 25 * 60_000,
  })
  const { wrapper } = await mountCard(idle([done]))
  expect(wrapper.find('[aria-label="Recent focus blocks"]').text()).toBe(
    'Done · 12 min · Write auth tests',
  )
})

it('closes the start form without starting anything', async () => {
  const { wrapper, focus } = await mountCard(idle())
  openFocusForm()
  expect(focus.formOpen.value).toBe(true)
  await button(wrapper, '×').trigger('click')
  expect(focus.formOpen.value).toBe(false)
  expect(calls('focus_start')).toHaveLength(0)
})
const proposals = [
  { title: 'Finish the report draft', doneWhen: 'Report draft is complete', minutes: 45 },
  { title: 'Outline the report', doneWhen: 'Every section has a heading', minutes: 20 },
  { title: 'Finish and review the report', doneWhen: 'Report is proofread', minutes: 75 },
]

async function suggestWith(result: unknown) {
  const mounted = await mountCard(idle(), () => focusing())
  api.invoke.mockImplementation(async (command) => {
    if (command === 'focus_suggest') {
      if (result instanceof Error) throw result.message
      return result
    }
    return command === 'focus_snapshot' ? idle() : focusing()
  })
  openFocusForm()
  await flushPromises()
  await mounted.wrapper.find('input').setValue('need to finish report and email bob')
  await button(mounted.wrapper, 'Suggest a block').trigger('click')
  await flushPromises()
  return mounted
}
const fields = (wrapper: ReturnType<typeof mount>) => ({
  title: (wrapper.findAll('input')[0]!.element as HTMLInputElement).value,
  doneWhen: (wrapper.findAll('input')[1]!.element as HTMLInputElement).value,
  length: wrapper.find('[aria-label="Block length"]').text(),
})

it('fills the form from the suggested block and sends the rough goal to the model', async () => {
  const { wrapper } = await suggestWith({ source: 'model', note: null, options: proposals })
  expect(calls('focus_suggest')[0]![1]).toEqual({ goal: 'need to finish report and email bob' })
  expect(fields(wrapper)).toEqual({
    title: 'Finish the report draft',
    doneWhen: 'Report draft is complete',
    length: '45 min',
  })
  expect(wrapper.find('[aria-label="Suggestion"]').text()).toContain('Suggested (1 of 3)')
})

it('cycles through the shorter and longer alternatives with Another', async () => {
  const { wrapper } = await suggestWith({ source: 'model', note: null, options: proposals })
  await button(wrapper, 'Another').trigger('click')
  expect(fields(wrapper).title).toBe('Outline the report')
  expect(fields(wrapper).length).toBe('20 min')
  expect(wrapper.text()).toContain('Shorter (2 of 3)')
  await button(wrapper, 'Another').trigger('click')
  expect(fields(wrapper).length).toBe('75 min')
  expect(wrapper.text()).toContain('Longer (3 of 3)')
  await button(wrapper, 'Another').trigger('click')
  expect(fields(wrapper).title).toBe('Finish the report draft')
})

it('lets you adjust a suggestion by five minutes and starts exactly what is shown', async () => {
  const { wrapper } = await suggestWith({ source: 'model', note: null, options: proposals })
  await button(wrapper, '+').trigger('click')
  await wrapper.find('form').trigger('submit')
  await flushPromises()
  expect(calls('focus_start')[0]![1]).toEqual({
    title: 'Finish the report draft',
    doneWhen: 'Report draft is complete',
    minutes: 50,
  })
})

it('uses the plain block and says why when no model answered', async () => {
  const plain = {
    source: 'fallback',
    note: 'The model could not suggest a block, so this is a plain 25-minute block.',
    options: [{ title: 'need to finish report and email bob', doneWhen: '', minutes: 25 }],
  }
  const { wrapper } = await suggestWith(plain)
  expect(wrapper.find('[aria-label="Suggestion"]').text()).toContain('plain 25-minute block')
  expect(fields(wrapper).length).toBe('25 min')
  expect(button(wrapper, 'Another')).toBeUndefined()
})

it('keeps what was typed and shows the reason when suggesting fails', async () => {
  const { wrapper } = await suggestWith(new Error('Keep it to 500 characters or fewer.'))
  expect(wrapper.find('[role=alert]').text()).toContain('500 characters')
  expect(fields(wrapper).title).toBe('need to finish report and email bob')
})
const meeting = (inMinutes: number, id = 1): FocusCommitment => ({
  id,
  title: 'Meeting',
  startMs: Date.now() + inMinutes * 60_000,
  endMs: Date.now() + (inMinutes + 30) * 60_000,
  source: 'manual',
})

it('shows free time until the next commitment, buffer kept clear', async () => {
  const { wrapper } = await mountCard(idle([], [meeting(40)]))
  expect(wrapper.find('[aria-label="Free time"]').text()).toMatch(/^Free: (34|35)m until Meeting$/)
})

it('warns when the block is longer than the free time', async () => {
  const { wrapper } = await mountCard(idle([], [meeting(20)]))
  expect(wrapper.find('[aria-label="Free time"]').text()).toContain('longer than your free time')
  for (let step = 0; step < 3; step++) await button(wrapper, '−').trigger('click')
  expect(wrapper.find('[aria-label="Free time"]').text()).not.toContain('longer')
})

it('shows no free time line when nothing is coming up', async () => {
  const { wrapper } = await mountCard(idle())
  expect(wrapper.find('[aria-label="Free time"]').exists()).toBe(false)
})

it('tells the running block to wrap up five minutes before a commitment and offers Extend', async () => {
  const { wrapper } = await mountCard(focusing(running(), [meeting(4)]), () => focusing())
  expect(wrapper.find('[aria-label="Wrap up"]').text()).toContain('Wrap up: Meeting starts at')
  await button(wrapper, 'Extend 5m').trigger('click')
  await flushPromises()
  expect(calls('focus_extend')).toHaveLength(1)
})

it('does not warn or offer Extend while there is plenty of time', async () => {
  const { wrapper } = await mountCard(focusing(running(), [meeting(40)]))
  expect(wrapper.find('[aria-label="Wrap up"]').exists()).toBe(false)
  expect(button(wrapper, 'Extend 5m')).toBeUndefined()
})

it('offers Extend once the block has run out, and shows why it is refused', async () => {
  const over = running({ endsMs: Date.now() - 1_000 })
  const { wrapper } = await mountCard(focusing(over))
  api.invoke.mockImplementation(async (command) => {
    if (command === 'focus_extend') throw 'No room to extend: "Meeting" starts in 9 min.'
    return focusing(over)
  })
  await button(wrapper, 'Extend 5m').trigger('click')
  await flushPromises()
  expect(wrapper.find('[role=alert]').text()).toContain('No room to extend')
})

it('adds a commitment from a title, a time and a length', async () => {
  const { wrapper } = await mountCard(idle())
  expect(button(wrapper, 'Add commitment').attributes('disabled')).toBeDefined()
  await wrapper.find('[aria-label="Commitment"]').setValue('Standup')
  await wrapper.find('[aria-label="Commitment starts at"]').setValue('23:59')
  await button(wrapper, 'Add commitment').trigger('click')
  await flushPromises()
  const args = calls('focus_commitment_add')[0]![1] as Record<string, unknown>
  expect(args.title).toBe('Standup')
  expect(args.minutes).toBe(30)
  expect(new Date(args.startMs as number).getHours()).toBe(23)
  expect(new Date(args.startMs as number).getMinutes()).toBe(59)
})

it('lists upcoming commitments and removes one', async () => {
  const { wrapper } = await mountCard(idle([], [meeting(40, 7)]), () => idle())
  expect(wrapper.find('[aria-label="Upcoming commitments"]').text()).toContain('Meeting')
  await wrapper.find('[aria-label="Remove Meeting"]').trigger('click')
  await flushPromises()
  expect(calls('focus_commitment_remove')[0]![1]).toEqual({ id: 7 })
})
it('suggests again from the original goal, not from the previous suggestion', async () => {
  const { wrapper } = await suggestWith({ source: 'model', note: null, options: proposals })
  await button(wrapper, 'Suggest a block').trigger('click')
  await flushPromises()
  const goals = calls('focus_suggest').map(([, args]) => (args as { goal: string }).goal)
  expect(goals).toEqual([
    'need to finish report and email bob',
    'need to finish report and email bob',
  ])
})

it('suggests from a new goal when the title was edited after a suggestion', async () => {
  const { wrapper } = await suggestWith({ source: 'model', note: null, options: proposals })
  await wrapper.find('input').setValue('plan the offsite')
  await button(wrapper, 'Suggest a block').trigger('click')
  await flushPromises()
  const goals = calls('focus_suggest').map(([, args]) => (args as { goal: string }).goal)
  expect(goals.at(-1)).toBe('plan the offsite')
})
const studentPreset = {
  id: 'student',
  label: 'Student',
  summary: 'Shorter blocks and time to get to class.',
  defaultMinutes: 25,
  capMinutes: 50,
  bufferMinutes: 10,
  wrapUpMinutes: 10,
  extendMinutes: 5,
  doneWhenLabel: 'What I will finish',
}
const asStudent = (snapshot: FocusSnapshot): FocusSnapshot => ({
  ...snapshot,
  preset: studentPreset,
  presets: [studentPreset],
})

it('uses the preset wording, the length cap and the buffer', async () => {
  const meeting: FocusCommitment = {
    id: 1,
    title: 'Class',
    startMs: Date.now() + 90 * 60_000,
    endMs: Date.now() + 120 * 60_000,
    source: 'manual',
  }
  const { wrapper } = await mountCard(asStudent(idle([], [meeting])))
  openFocusForm()
  await flushPromises()
  expect(wrapper.find('input[aria-label="What I will finish"]').exists()).toBe(true)
  for (let i = 0; i < 8; i++) await button(wrapper, '+').trigger('click')
  expect(wrapper.text()).toContain('50')
  expect(button(wrapper, '+').attributes('disabled')).toBeDefined()
  expect(wrapper.text()).toContain('Free: 1h 19m until Class')
})

it('names the done-when line with the preset wording while focusing', async () => {
  const { wrapper } = await mountCard(asStudent(focusing()))
  expect(wrapper.text()).toContain('What I will finish: tests pass')
})
it('marks a calendar meeting and gives it no remove button', async () => {
  const synced: FocusCommitment = { ...meeting(60, 2), title: 'Design review', source: 'calendar' }
  const { wrapper } = await mountCard(idle([], [meeting(40), synced]))
  const list = wrapper.get('[aria-label="Upcoming commitments"]')
  expect(list.text()).toContain('Design review')
  expect(list.text()).toContain('calendar')
  expect(list.find('button[aria-label="Remove Design review"]').exists()).toBe(false)
  expect(list.find('button[aria-label="Remove Meeting"]').exists()).toBe(true)
})

it('syncs the calendar on request and says what was read', async () => {
  const { wrapper } = await mountCard(idle(), () => idle())
  api.invoke.mockImplementation(async (command) =>
    command === 'focus_calendar_sync'
      ? { state: 'synced', events: 2, message: 'Calendar read: 2 events in the next 48 hours.' }
      : idle(),
  )
  await button(wrapper, 'Sync calendar').trigger('click')
  await flushPromises()
  expect(calls('focus_calendar_sync')).toHaveLength(1)
  expect(wrapper.text()).toContain('Calendar read: 2 events in the next 48 hours.')
})

it('says why there is no calendar instead of failing', async () => {
  const { wrapper } = await mountCard(idle())
  api.invoke.mockImplementation(async (command) =>
    command === 'focus_calendar_sync'
      ? {
          state: 'unavailable',
          events: 0,
          message: 'No calendar tool is selected. Add a calendar in Tools settings.',
        }
      : idle(),
  )
  await button(wrapper, 'Sync calendar').trigger('click')
  await flushPromises()
  expect(wrapper.get('[aria-label="Calendar sync"]').text()).toContain(
    'No calendar tool is selected',
  )
})
it('opens the start form and puts the cursor in the goal when the hotkey is pressed', async () => {
  const { wrapper, focus } = await mountCard(idle(), undefined, document.body)
  expect(focus.formOpen.value).toBe(false)
  fire('focus_hotkey')
  await flushPromises()
  expect(focus.formOpen.value).toBe(true)
  expect(document.activeElement).toBe(
    wrapper.get('input[aria-label="What will you work on?"]').element,
  )
  wrapper.unmount()
})

it('puts the keyboard on Done when the hotkey is pressed during a block', async () => {
  const wrapper = (await mountCard(focusing(), undefined, document.body)).wrapper
  fire('focus_hotkey')
  await flushPromises()
  expect(document.activeElement?.textContent?.trim()).toBe('Done')
  wrapper.unmount()
})

const mic = (wrapper: ReturnType<typeof mount>) =>
  wrapper.get('button[aria-label="Say what you will work on"], button[aria-label="Stop listening"]')

it('turns what is said into the goal, asks for a suggestion and stops listening', async () => {
  const { wrapper } = await mountCard(idle(), () => idle())
  api.invoke.mockImplementation(async (command) =>
    command === 'focus_suggest'
      ? { source: 'model', note: null, options: proposals }
      : command === 'focus_snapshot'
        ? idle()
        : null,
  )
  openFocusForm()
  await flushPromises()
  await mic(wrapper).trigger('click')
  await flushPromises()
  expect(calls('focus_dictation_set')[0]![1]).toEqual({ on: true })
  expect(mic(wrapper).attributes('aria-pressed')).toBe('true')
  fire('focus_dictated', '  need to finish the report  ')
  await flushPromises()
  expect(calls('focus_dictation_set')[1]![1]).toEqual({ on: false })
  expect(calls('focus_suggest')[0]![1]).toEqual({ goal: 'need to finish the report' })
  expect(mic(wrapper).attributes('aria-pressed')).toBe('false')
})

it('ignores words that arrive when the card is not listening', async () => {
  const { wrapper } = await mountCard(idle())
  openFocusForm()
  await flushPromises()
  fire('focus_dictated', 'open notepad')
  await flushPromises()
  expect(calls('focus_suggest')).toHaveLength(0)
  expect((wrapper.get('input').element as HTMLInputElement).value).toBe('')
})

it('shows why listening could not start and stays off', async () => {
  const { wrapper } = await mountCard(idle())
  api.invoke.mockImplementation(async (command) => {
    if (command === 'focus_dictation_set') throw 'Configure a Voice model in the Infer catalog'
    return command === 'focus_snapshot' ? idle() : null
  })
  openFocusForm()
  await flushPromises()
  await mic(wrapper).trigger('click')
  await flushPromises()
  expect(wrapper.get('[role=alert]').text()).toContain('Voice model')
  expect(mic(wrapper).attributes('aria-pressed')).toBe('false')
})

it('stops listening when the form is closed', async () => {
  const { wrapper, focus } = await mountCard(idle())
  openFocusForm()
  await flushPromises()
  await mic(wrapper).trigger('click')
  await flushPromises()
  focus.closeForm()
  await flushPromises()
  expect(calls('focus_dictation_set').map(([, args]) => args)).toEqual([
    { on: true },
    { on: false },
  ])
})

it('suggests with Ctrl+Enter and starts with plain Enter', async () => {
  const { wrapper } = await mountCard(idle(), () => focusing())
  api.invoke.mockImplementation(async (command) =>
    command === 'focus_suggest'
      ? { source: 'model', note: null, options: proposals }
      : command === 'focus_snapshot'
        ? idle()
        : focusing(),
  )
  openFocusForm()
  await flushPromises()
  const title = wrapper.get('input[aria-label="What will you work on?"]')
  await title.setValue('report')
  await title.trigger('keydown', { key: 'Enter', ctrlKey: true })
  await flushPromises()
  expect(calls('focus_suggest')).toHaveLength(1)
  expect(calls('focus_start')).toHaveLength(0)
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(calls('focus_start')).toHaveLength(1)
})

it('finishes the block with Ctrl+Enter and extends with Ctrl+E when there is a reason to', async () => {
  const over = running({ endsMs: Date.now() - 1_000 })
  const { wrapper } = await mountCard(focusing(over), (command) =>
    command === 'focus_finish' ? idle() : focusing(over),
  )
  await wrapper.get('[aria-label="Focus block"]').trigger('keydown', { key: 'e', ctrlKey: true })
  await flushPromises()
  expect(calls('focus_extend')).toHaveLength(1)
  await wrapper
    .get('[aria-label="Focus block"]')
    .trigger('keydown', { key: 'Enter', ctrlKey: true })
  await flushPromises()
  expect(calls('focus_finish')[0]![1]).toEqual({ outcome: 'done' })
})

it('does not extend with Ctrl+E while plenty of time is left', async () => {
  const { wrapper } = await mountCard(focusing())
  await wrapper.get('[aria-label="Focus block"]').trigger('keydown', { key: 'e', ctrlKey: true })
  await flushPromises()
  expect(calls('focus_extend')).toHaveLength(0)
})
it('hands the running block to the agent once and then says it is with the agent', async () => {
  const handed = running({ handedOffMs: Date.now() })
  const { wrapper } = await mountCard(focusing(), (command) =>
    command === 'focus_hand_off' ? focusing(handed) : focusing(),
  )
  expect(wrapper.find('[aria-label="Handed to the agent"]').exists()).toBe(false)
  await button(wrapper, 'Hand to agent').trigger('click')
  await flushPromises()
  expect(calls('focus_hand_off')).toHaveLength(1)
  expect(wrapper.get('[aria-label="Handed to the agent"]').text()).toContain('With the agent')
  expect(button(wrapper, 'Hand to agent')).toBeUndefined()
})

it('shows why the hand-off was refused and keeps the button', async () => {
  const { wrapper } = await mountCard(focusing())
  api.invoke.mockImplementation(async (command) => {
    if (command === 'focus_hand_off') throw 'Start tasks in your external agent host'
    return focusing()
  })
  await button(wrapper, 'Hand to agent').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role=alert]').text()).toContain('external agent host')
  expect(button(wrapper, 'Hand to agent')).toBeDefined()
})

it('offers no hand-off for a block that is already with the agent', async () => {
  const { wrapper } = await mountCard(focusing(running({ handedOffMs: Date.now() })))
  expect(button(wrapper, 'Hand to agent')).toBeUndefined()
  expect(wrapper.find('[aria-label="Handed to the agent"]').exists()).toBe(true)
})