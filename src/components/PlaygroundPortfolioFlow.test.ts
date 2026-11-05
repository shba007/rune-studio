import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import PlaygroundPortfolioFlow from './PlaygroundPortfolioFlow.vue'

const testModels = [
  { id: 'vision', imageIn: 'yes', toolCalls: 'yes', locality: 'local' },
  {
    id: 'matte',
    locality: 'local',
    tasks: ['text-image-to-image'],
    rawCapabilities: ['Transparency (RGBA)'],
  },
] as never
let classification = 'uncertain'
async function configureModels(wrapper: ReturnType<typeof mount>) {
  await wrapper.get('#portfolio-vision-model').setValue('vision')
  await wrapper.get('#portfolio-edit-model').setValue('matte')
}

const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({
  invoke: async (command: string, args?: unknown) => {
    const result = await invoke(command, args)
    if (result === undefined && command === 'portfolio_developed_preview')
      return 'data:image/png;base64,YQ=='
    if (result === undefined && command === 'portfolio_suggest_look')
      return { exposure: 0, warmth: 0 }
    return result
  },
}))

function file(name: string, path: string) {
  const result = new File(['raw'], name)
  Object.defineProperty(result, 'webkitRelativePath', { value: path })
  return result
}

describe('Portfolio Studio', () => {
  beforeEach(() => {
    classification = 'uncertain'
    invoke.mockReset()
    invoke.mockImplementation(async (command: string) => {
      if (command === 'portfolio_raw_preview') return 'data:image/jpeg;base64,YQ=='
      if (command === 'playground_start_run') return 1
      if (command === 'portfolio_develop_raw')
        return { path: 'C:\\Exports\\developed.png', width: 4672, height: 7008 }
      if (command === 'portfolio_classify') return classification
      if (command === 'portfolio_review') return 'DECISION: ACCEPT\nCheck the edges by hand.'
      if (command === 'portfolio_finish_preview')
        return { original: 'data:image/png;base64,YQ==', composite: 'data:image/png;base64,Yw==' }
      if (command === 'playground_extract_subject')
        return {
          original: 'data:image/png;base64,YQ==',
          cutout: 'data:image/png;base64,Yg==',
          composite: 'data:image/png;base64,Yw==',
          model: 'matte',
          elapsedMs: 1000,
        }
      return undefined
    })
  })

  it('develops the selected RAW and reviews the estimated shared look', async () => {
    classification = 'front'
    const fallback = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command, args) =>
      command === 'portfolio_suggest_look'
        ? { exposure: 25, warmth: -20 }
        : fallback(command, args),
    )
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    Object.defineProperty(wrapper.get('#portfolio-folder').element, 'files', {
      value: [file('one.ARW', 'shoot/one.ARW')],
    })
    await wrapper.get('#portfolio-folder').trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_suggest_look', expect.anything()),
    )
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_finish_preview', {
        input: expect.objectContaining({
          settings: expect.objectContaining({ exposure: 25, warmth: -20 }),
        }),
      }),
    )
    expect(
      invoke.mock.calls.findIndex(([command]) => command === 'portfolio_developed_preview'),
    ).toBeLessThan(
      invoke.mock.calls.findIndex(([command]) => command === 'playground_extract_subject'),
    )
    expect(
      invoke.mock.calls.findIndex(([command]) => command === 'portfolio_suggest_look'),
    ).toBeLessThan(invoke.mock.calls.findIndex(([command]) => command === 'portfolio_review'))
  })

  it('stops before vision review when automatic look estimation fails', async () => {
    classification = 'front'
    const fallback = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command, args) => {
      if (command === 'portfolio_suggest_look') throw new Error('Not enough visible subject pixels')
      return fallback(command, args)
    })
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    Object.defineProperty(wrapper.get('#portfolio-folder').element, 'files', {
      value: [file('one.ARW', 'shoot/one.ARW')],
    })
    await wrapper.get('#portfolio-folder').trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.text()).toContain('Turn off automatic look to use manual adjustments.'),
    )
    expect(invoke).not.toHaveBeenCalledWith('portfolio_review', expect.anything())
    expect(
      wrapper
        .findAll('input[type="checkbox"]')
        .some((input) => !(input.element as HTMLInputElement).disabled),
    ).toBe(true)
  })

  it('offers only verified local portrait matting models alongside RGBA editors', () => {
    const wrapper = mount(PlaygroundPortfolioFlow, {
      props: {
        models: [
          {
            id: 'verified',
            locality: 'local',
            tasks: ['image-to-matting'],
            portraitMattingVerified: true,
          },
          {
            id: 'unverified',
            locality: 'local',
            tasks: ['image-to-matting'],
            portraitMattingVerified: false,
          },
          {
            id: 'remote',
            locality: 'cloud',
            tasks: ['image-to-matting'],
            portraitMattingVerified: true,
          },
          {
            id: 'rgba',
            locality: 'local',
            tasks: ['text-image-to-image'],
            rawCapabilities: ['Transparency (RGBA)'],
          },
        ] as never,
      },
    })
    const options = wrapper
      .get('#portfolio-edit-model')
      .findAll('option')
      .map((option) => option.element.value)
    expect(options).toEqual(['', 'verified', 'rgba'])
  })

  it('does not repeat a deterministic matte when vision requests a retry', async () => {
    classification = 'front'
    const fallback = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command, args) =>
      command === 'portfolio_review' ? 'DECISION: RETRY\nCheck the edge.' : fallback(command, args),
    )
    const wrapper = mount(PlaygroundPortfolioFlow, {
      props: {
        models: [
          { id: 'vision', imageIn: 'yes', toolCalls: 'yes', locality: 'local' },
          {
            id: 'matte',
            locality: 'local',
            tasks: ['image-to-matting'],
            portraitMattingVerified: true,
          },
        ] as never,
      },
    })
    Object.defineProperty(wrapper.get('#portfolio-folder').element, 'files', {
      value: [file('one.ARW', 'shoot/one.ARW')],
    })
    await wrapper.get('#portfolio-folder').trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_review', expect.anything()),
    )
    await flushPromises()
    expect(
      invoke.mock.calls.filter(([command]) => command === 'playground_extract_subject'),
    ).toHaveLength(1)
  })

  it('exports the selected RAW with the verified matte and background settings', async () => {
    classification = 'front'
    const fallback = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command, args) =>
      command === 'portfolio_export_composite'
        ? { path: 'C:\\Exports\\composite.png', width: 4672, height: 7008 }
        : fallback(command, args),
    )
    const wrapper = mount(PlaygroundPortfolioFlow, {
      props: {
        models: [
          { id: 'vision', imageIn: 'yes', toolCalls: 'yes', locality: 'local' },
          {
            id: 'matte',
            locality: 'local',
            tasks: ['image-to-matting'],
            portraitMattingVerified: true,
          },
        ] as never,
      },
    })
    Object.defineProperty(wrapper.get('#portfolio-folder').element, 'files', {
      value: [file('one.ARW', 'shoot/one.ARW')],
    })
    await wrapper.get('#portfolio-folder').trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(
        wrapper
          .findAll('button')
          .some(
            (button) =>
              button.text() === 'Export full-resolution composite' &&
              !button.attributes('disabled'),
          ),
      ).toBe(true),
    )
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Export full-resolution composite')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_export_composite', {
        runId: 1,
        input: {
          raw: expect.any(String),
          model: 'matte',
          settings: {
            crop: 'original',
            autoCenter: true,
            scale: 100,
            horizontal: 0,
            vertical: 0,
            exposure: 0,
            warmth: 0,
            backgroundLeft: 160,
            backgroundRight: 135,
          },
        },
      }),
    )
    await vi.waitFor(() => expect(wrapper.text()).toContain('4672 × 7008 PNG'))
  })

  it('blocks sidecars, prepares only eligible RAW files, and permits review', async () => {
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', {
      value: [
        file('one.ARW', 'shoot/one.ARW'),
        file('two.ARW', 'shoot/two.ARW'),
        file('two.xmp', 'shoot/two.xmp'),
      ],
    })
    await folder.trigger('change')
    await configureModels(wrapper)
    expect(wrapper.text()).toContain('1 files blocked')
    expect(wrapper.text()).toContain('shoot/two.ARW')
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() => {
      expect(
        invoke.mock.calls.filter(([command]) => command === 'portfolio_raw_preview'),
      ).toHaveLength(1)
    })
    await flushPromises()
    expect(wrapper.find('[aria-label="Choose views"]').exists()).toBe(true)
    await wrapper.get('[aria-label="front photo"]').setValue('shoot/one.ARW')
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Continue automatically')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true),
    )
    expect(wrapper.find('[aria-label="Subject cutout"]').exists()).toBe(true)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Develop 16-bit PNG')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_develop_raw', { raw: expect.any(String) }),
    )
    await flushPromises()
    expect(wrapper.text()).toContain('4672 × 7008 PNG')
    expect(wrapper.text()).toContain('C:\\Exports\\developed.png')
  })

  it('rejects a selection without folder paths', async () => {
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', '')] })
    await folder.trigger('change')
    expect(wrapper.text()).toContain('Choose a folder so Studio can check')
    expect(invoke).not.toHaveBeenCalled()
  })

  it('requires both models and automatically classifies, creates and reviews without approval', async () => {
    classification = 'front'
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    const run = wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
    expect(run.attributes('disabled')).toBeDefined()
    await configureModels(wrapper)
    expect(run.attributes('disabled')).toBeUndefined()
    await run.trigger('click')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_review', expect.anything()),
    )
    await vi.waitFor(() =>
      expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true),
    )
    expect(wrapper.text()).toContain('Check the edges by hand.')
    expect(wrapper.find('a[download="portfolio-cutout-preview.png"]').exists()).toBe(true)
    expect(invoke.mock.calls.map(([command]) => command)).toContain('playground_extract_subject')
  })

  it('stops the automatic pipeline and ignores a late cutout', async () => {
    let complete!: (value: unknown) => void
    invoke.mockImplementation(async (command: string) => {
      if (command === 'portfolio_raw_preview') return 'data:image/jpeg;base64,YQ=='
      if (command === 'playground_start_run') return 42
      if (command === 'portfolio_classify') return 'front'
      if (command === 'playground_extract_subject')
        return new Promise((resolve) => {
          complete = resolve
        })
      return undefined
    })
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() => expect(complete).toBeTypeOf('function'))
    await wrapper.get('[data-testid="result-cancel"]').trigger('click')
    complete({
      original: 'data:image/png;base64,YQ==',
      cutout: 'data:image/png;base64,Yg==',
      composite: 'data:image/png;base64,Yw==',
      model: 'matte',
      elapsedMs: 1000,
    })
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('playground_cancel_run', { runId: 42 })
    expect(invoke).not.toHaveBeenCalledWith('portfolio_review', expect.anything())
    expect(wrapper.find('[alt="Studio background preview"]').exists()).toBe(false)
  })

  it('retries a failed cutout once and then reviews the successful result', async () => {
    let attempts = 0
    invoke.mockImplementation(async (command: string) => {
      if (command === 'portfolio_raw_preview') return 'data:image/jpeg;base64,YQ=='
      if (command === 'playground_start_run') return 1
      if (command === 'portfolio_classify') return 'front'
      if (command === 'playground_extract_subject') {
        attempts++
        if (attempts === 1) throw new Error('The returned subject mask is mostly translucent')
        return {
          original: 'data:image/png;base64,YQ==',
          cutout: 'data:image/png;base64,Yg==',
          composite: 'data:image/png;base64,Yw==',
          model: 'matte',
          elapsedMs: 1000,
        }
      }
      if (command === 'portfolio_finish_preview')
        return { original: 'data:image/png;base64,YQ==', composite: 'data:image/png;base64,Yw==' }
      if (command === 'portfolio_review') return 'DECISION: ACCEPT\nReview notes.'
      return undefined
    })
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true),
    )
    const calls = invoke.mock.calls.filter(([command]) => command === 'playground_extract_subject')
    expect(calls).toHaveLength(2)
    expect(calls[1]![1]).toEqual(
      expect.objectContaining({ input: expect.objectContaining({ retry: true }) }),
    )
  })

  it('retries once when the vision model requests an edit', async () => {
    let reviews = 0
    invoke.mockImplementation(async (command: string) => {
      if (command === 'portfolio_raw_preview') return 'data:image/jpeg;base64,YQ=='
      if (command === 'playground_start_run') return 1
      if (command === 'portfolio_classify') return 'front'
      if (command === 'playground_extract_subject')
        return {
          original: 'data:image/png;base64,YQ==',
          cutout: 'data:image/png;base64,Yg==',
          composite: 'data:image/png;base64,Yw==',
          model: 'matte',
          elapsedMs: 1000,
        }
      if (command === 'portfolio_finish_preview')
        return { original: 'data:image/png;base64,YQ==', composite: 'data:image/png;base64,Yw==' }
      if (command === 'portfolio_review')
        return ++reviews === 1
          ? 'DECISION: RETRY\nFix the background.'
          : 'DECISION: ACCEPT\nLooks consistent.'
      return undefined
    })
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true),
    )
    expect(
      invoke.mock.calls.filter(([command]) => command === 'playground_extract_subject'),
    ).toHaveLength(2)
    expect(reviews).toBe(2)
  })

  it('asks for review when the model gives no recognizable decision', async () => {
    classification = 'front'
    const base = invoke.getMockImplementation()!
    invoke.mockImplementation(async (command, args) =>
      command === 'portfolio_review' ? 'Looks fine.' : base(command, args),
    )
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() => expect(wrapper.text()).toContain('vision model could not confirm'))
    expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(false)
  })

  it('keeps both model selectors in settings and locks them during preparation', async () => {
    let finishPreview!: (value: string) => void
    invoke.mockImplementation(async (command: string) => {
      if (command === 'portfolio_raw_preview')
        return new Promise<string>((resolve) => {
          finishPreview = resolve
        })
      return undefined
    })
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const settings = wrapper.get('[aria-label="Portfolio Studio settings"]')
    expect(settings.find('#portfolio-vision-model').exists()).toBe(true)
    expect(settings.find('#portfolio-edit-model').exists()).toBe(true)
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() => expect(finishPreview).toBeTypeOf('function'))
    expect(settings.get('#portfolio-vision-model').attributes('disabled')).toBeDefined()
    expect(settings.get('#portfolio-edit-model').attributes('disabled')).toBeDefined()
    finishPreview('data:image/jpeg;base64,YQ==')
    await vi.waitFor(() =>
      expect(settings.get('#portfolio-edit-model').attributes('disabled')).toBeUndefined(),
    )
  })

  it('guides the user through photo selection and keeps their work when navigating back', async () => {
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    expect(wrapper.get('[aria-label="Import photos"]').text()).toContain('Choose shoot folder')
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('[aria-label="Assign one.ARW to front"]').exists()).toBe(true),
    )
    const continueButton = wrapper
      .findAll('button')
      .find((button) => button.text() === 'Continue automatically')!
    expect(continueButton.attributes('disabled')).toBeDefined()
    await wrapper.get('[aria-label="Assign one.ARW to front"]').trigger('click')
    expect((wrapper.get('[aria-label="front photo"]').element as HTMLSelectElement).value).toBe(
      'shoot/one.ARW',
    )
    await continueButton.trigger('click')
    expect(wrapper.find('[aria-label="Subject cutout"]').exists()).toBe(true)
    await vi.waitFor(() =>
      expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true),
    )
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Edit selections')!
      .trigger('click')
    expect((wrapper.get('[aria-label="front photo"]').element as HTMLSelectElement).value).toBe(
      'shoot/one.ARW',
    )
  })

  it('lets the user prepare missing photos without losing a selection', async () => {
    let calls = 0
    invoke.mockImplementation(async (command: string) => {
      if (command === 'portfolio_raw_preview') {
        calls++
        if (calls === 2) throw new Error('Preview unavailable')
        return 'data:image/jpeg;base64,YQ=='
      }
      if (command === 'playground_start_run') return 1
      if (command === 'portfolio_classify') return 'uncertain'
      if (command === 'playground_extract_subject')
        return {
          original: 'data:image/png;base64,YQ==',
          cutout: 'data:image/png;base64,Yg==',
          composite: 'data:image/png;base64,Yw==',
          model: 'matte',
          elapsedMs: 1000,
        }
      if (command === 'portfolio_finish_preview')
        return { original: 'data:image/png;base64,YQ==', composite: 'data:image/png;base64,Yw==' }
      if (command === 'portfolio_review') return 'DECISION: ACCEPT\nCheck edges.'
      return undefined
    })
    const wrapper = mount(PlaygroundPortfolioFlow, { props: { models: testModels } })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', {
      value: [file('one.ARW', 'shoot/one.ARW'), file('two.ARW', 'shoot/two.ARW')],
    })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('[aria-label="Assign one.ARW to front"]').exists()).toBe(true),
    )
    await wrapper.get('[aria-label="Assign one.ARW to front"]').trigger('click')
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Prepare remaining photos')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('[aria-label="Subject cutout"]').exists()).toBe(true),
    )
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Edit selections')!
      .trigger('click')
    expect(wrapper.find('[aria-label="Assign two.ARW to front"]').exists()).toBe(true)
    expect((wrapper.get('[aria-label="front photo"]').element as HTMLSelectElement).value).toBe(
      'shoot/one.ARW',
    )
  })

  it('uses a local vision model to suggest a view while retaining manual control', async () => {
    classification = 'front\nFacing the camera.'
    const wrapper = mount(PlaygroundPortfolioFlow, {
      props: {
        models: testModels,
      },
    })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('portfolio_classify', expect.anything()),
    )
    await vi.waitFor(() =>
      expect(wrapper.find('[aria-label="Subject cutout"]').exists()).toBe(true),
    )
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Edit selections')!
      .trigger('click')
    expect(wrapper.text()).toContain('Suggested: front')
    expect((wrapper.get('[aria-label="front photo"]').element as HTMLSelectElement).value).toBe(
      'shoot/one.ARW',
    )
    await wrapper.get('[aria-label="front photo"]').setValue('')
    expect((wrapper.get('[aria-label="front photo"]').element as HTMLSelectElement).value).toBe('')
  })

  it('continues automatically after manual selection and retains optional review and retry', async () => {
    const wrapper = mount(PlaygroundPortfolioFlow, {
      props: {
        models: [
          {
            id: 'matte',
            locality: 'local',
            tasks: ['text-image-to-image'],
            rawCapabilities: ['Transparency (RGBA)'],
          },
          { id: 'vision', imageIn: 'yes', toolCalls: 'yes', locality: 'local' },
        ] as never,
      },
    })
    const folder = wrapper.get('#portfolio-folder')
    Object.defineProperty(folder.element, 'files', { value: [file('one.ARW', 'shoot/one.ARW')] })
    await folder.trigger('change')
    await configureModels(wrapper)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Run automated previews')!
      .trigger('click')
    await vi.waitFor(() => expect(wrapper.find('[aria-label="front photo"]').exists()).toBe(true))
    await wrapper.get('[aria-label="front photo"]').setValue('shoot/one.ARW')
    expect(wrapper.find('[aria-label="Subject cutout"]').exists()).toBe(false)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Continue automatically')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true),
    )
    expect(invoke).toHaveBeenCalledWith(
      'playground_extract_subject',
      expect.objectContaining({
        input: { image: 'data:image/png;base64,YQ==', model: 'matte', retry: false },
      }),
    )
    await vi.waitFor(() =>
      expect(wrapper.find('[alt="Studio background preview"]').exists()).toBe(true),
    )
    expect(invoke).toHaveBeenCalledWith(
      'portfolio_review',
      expect.objectContaining({
        input: {
          original: 'data:image/png;base64,YQ==',
          candidate: 'data:image/png;base64,Yw==',
          model: 'vision',
        },
      }),
    )
    expect(wrapper.text()).toContain('Check the edges by hand.')
    expect(wrapper.find('a[download="portfolio-look-preview.png"]').exists()).toBe(true)
    invoke.mockResolvedValueOnce(2).mockResolvedValueOnce({
      original: 'data:image/png;base64,YQ==',
      cutout: 'data:image/png;base64,eA==',
      composite: 'data:image/png;base64,eQ==',
      model: 'matte',
      elapsedMs: 1000,
    })
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Retry from original')!
      .trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith(
      'playground_extract_subject',
      expect.objectContaining({
        input: { image: 'data:image/png;base64,YQ==', model: 'matte', retry: true },
      }),
    )
    expect(
      wrapper
        .findAll('button')
        .find((button) => button.text() === 'Retry from original')!
        .attributes('disabled'),
    ).toBeDefined()
    expect(
      wrapper.findAll('button').some((button) => button.text() === 'Show other candidate'),
    ).toBe(true)
    await vi.waitFor(() =>
      expect(wrapper.find('[alt="Studio background preview"]').exists()).toBe(true),
    )
    expect(invoke.mock.calls.filter(([command]) => command === 'portfolio_review')).toHaveLength(2)
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Show other candidate')!
      .trigger('click')
    await vi.waitFor(() =>
      expect(wrapper.find('[alt="Studio background preview"]').exists()).toBe(true),
    )
    expect(wrapper.text()).toContain('Check the edges by hand.')
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Mark reviewed (optional)')!
      .trigger('click')
    expect(wrapper.get('a[download="portfolio-cutout-preview.png"]').attributes('href')).toBe(
      'data:image/png;base64,Yg==',
    )
    expect(wrapper.get('a[download="portfolio-look-preview.png"]').attributes('href')).toBe(
      'data:image/png;base64,Yw==',
    )
    await wrapper.get('[aria-label="Exposure"]').setValue('30')
    expect(wrapper.find('a[download]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('Check the edges by hand.')
    await vi.waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'portfolio_finish_preview',
        expect.objectContaining({
          input: expect.objectContaining({ settings: expect.objectContaining({ exposure: 30 }) }),
        }),
      ),
    )
  })
})