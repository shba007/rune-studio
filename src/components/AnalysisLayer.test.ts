import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import AnalysisLayer from './AnalysisLayer.vue'

it.each([
  { id: 'replay-pose', scaleFactor: 1, x: 0, y: 0, expectedX: 200 },
  { id: 'insight-pose', scaleFactor: 2, x: -100, y: 50, expectedX: 50 },
])(
  'renders pose-only findings in $id and honors Structure visibility',
  async ({ id, scaleFactor, x, y, expectedX }) => {
    const snapshot = {
      ...EMPTY_SNAPSHOT,
      id,
      status: 'done' as const,
      monitor: { id: 1, x, y, width: 800, height: 400, scaleFactor: 1, primary: true },
      findings: [
        {
          n: 1,
          label: 'Arm pose',
          kind: 'image' as const,
          group: 'structure' as const,
          box: null,
          point: null,
          description: '',
          evidence: '',
          pose: {
            keypoints: [
              { label: 'shoulder', x: 250, y: 250 },
              { label: 'elbow', x: 500, y: 500 },
            ],
            connections: [[0, 1] as [number, number]],
          },
        },
      ],
    }
    const wrapper = mount(AnalysisLayer, {
      props: {
        snapshot,
        view: { width: 800, height: 400, scaleFactor },
        scanKey: 0,
      },
    })
    expect(
      Number.parseFloat((wrapper.get('[data-marker="1"]').element as HTMLElement).style.left),
    ).toBe(expectedX)
    expect(wrapper.findAll('[data-keypoint]')).toHaveLength(2)
    expect(wrapper.get('[data-pose-edge]').attributes('x2')).toBe(String(200 / scaleFactor))
    await wrapper.setProps({
      snapshot: {
        ...snapshot,
        visibleGroups: { entities: true, regions: true, structure: false, depth: true },
      },
    })
    expect(wrapper.find('[data-pose]').exists()).toBe(false)
    wrapper.unmount()
  },
)

it('places captured-window findings at the window origin and labels them as observations', () => {
  const wrapper = mount(AnalysisLayer, {
    props: {
      snapshot: {
        ...EMPTY_SNAPSHOT,
        id: 'insight-test',
        status: 'done',
        monitor: { id: 1, x: 200, y: 100, width: 800, height: 600, scaleFactor: 1, primary: false },
        findings: [
          {
            n: 1,
            label: 'Error',
            kind: 'issue',
            box: [0, 0, 500, 500],
            point: null,
            description: '',
            evidence: '',
          },
        ],
      },
      view: { width: 1920, height: 1080, scaleFactor: 1, originX: 0, originY: 0 },
      scanKey: 0,
    },
  })
  const marker = wrapper.get('[data-marker="1"]')
  expect(marker.attributes('style')).toContain('left: 200px')
  expect(marker.attributes('style')).toContain('top: 100px')
  expect(marker.attributes('style')).toContain('width: 400px')
  expect(marker.text()).toContain('Error')
  expect(wrapper.text()).toContain('Captured observations')
  wrapper.unmount()
})

it.each([
  { label: 'Full-body model', box: [320, 120, 430, 870], rect: [614.4, 129.6, 211.2, 810] },
  { label: 'Model face', box: [355, 120, 400, 200], rect: [681.6, 129.6, 86.4, 86.4] },
  { label: 'Model hands', box: [320, 440, 430, 540], rect: [614.4, 475.2, 211.2, 108] },
  { label: 'Image Preview tab', box: [130, 36, 210, 60], rect: [249.6, 38.88, 153.6, 25.92] },
])('renders the recorded $label box without adding an offset', ({ label, box, rect }) => {
  const wrapper = mount(AnalysisLayer, {
    props: {
      snapshot: {
        ...EMPTY_SNAPSHOT,
        id: 'insight-coordinate-regression',
        status: 'done',
        monitor: { id: 1, x: 0, y: 0, width: 1920, height: 1080, scaleFactor: 1, primary: true },
        findings: [
          {
            n: 1,
            label,
            kind: 'image',
            box: box as [number, number, number, number],
            point: null,
            description: '',
            evidence: '',
          },
        ],
      },
      view: { width: 1920, height: 1080, scaleFactor: 1, originX: 0, originY: 0 },
      scanKey: 0,
    },
  })
  const element = wrapper.get('[data-marker="1"]').element as HTMLElement
  for (const [index, property] of ['left', 'top', 'width', 'height'].entries()) {
    expect(Number.parseFloat(element.style.getPropertyValue(property))).toBeCloseTo(rect[index]!, 5)
  }
  wrapper.unmount()
})

it.each([
  { scale: 1.25, originX: -1920, originY: -200, expected: [320, 200, 384, 216] },
  { scale: 1.5, originX: -1920, originY: -200, expected: [400 / 1.5, 250 / 1.5, 320, 180] },
])(
  'maps a captured window on a negative-origin display at $scale DPI',
  ({ scale, originX, originY, expected }) => {
    const wrapper = mount(AnalysisLayer, {
      props: {
        snapshot: {
          ...EMPTY_SNAPSHOT,
          id: 'insight-window-dpi',
          status: 'done',
          monitor: {
            id: 2,
            x: -1760,
            y: -85,
            width: 960,
            height: 540,
            scaleFactor: 1,
            primary: false,
          },
          findings: [
            {
              n: 1,
              label: 'Known rectangle',
              kind: 'region',
              box: [250, 250, 750, 750],
              point: null,
              description: '',
              evidence: '',
            },
          ],
        },
        view: { width: 1920, height: 1080, scaleFactor: scale, originX, originY },
        scanKey: 0,
      },
    })
    const element = wrapper.get('[data-marker="1"]').element as HTMLElement
    for (const [index, property] of ['left', 'top', 'width', 'height'].entries()) {
      expect(Number.parseFloat(element.style.getPropertyValue(property))).toBeCloseTo(
        expected[index]!,
        5,
      )
    }
    wrapper.unmount()
  },
)