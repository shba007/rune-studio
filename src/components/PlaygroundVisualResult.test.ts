import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import PlaygroundVisualResult from './PlaygroundVisualResult.vue'

it('uses standard findings and annotations for reliable pose points', async () => {
  const wrapper = mount(PlaygroundVisualResult, {
    props: {
      image: 'data:image/png;base64,AA==',
      result: {
        model: 'pose',
        data: [
          {
            label: 'person',
            confidence: 0.9,
            pose: [
              { name: 'left_shoulder', x: 0.25, y: 0.4, confidence: 0.9 },
              { name: 'right_shoulder', x: 0.75, y: 0.4, confidence: 0.8 },
              { name: 'left_elbow', x: 0.2, y: 0.6, confidence: 0.01 },
            ],
          },
        ],
      },
    },
  })
  expect(wrapper.findAll('[data-keypoint]')).toHaveLength(2)
  expect(wrapper.findAll('svg line')).toHaveLength(1)
  expect(wrapper.get('[data-keypoint="left shoulder"]').attributes('style')).toContain('25%')
  await wrapper.get('[aria-label="Finding 1: person"]').trigger('click')
  expect(wrapper.text()).toContain('left elbow: 1%')
  expect(wrapper.text()).toContain('at least 50% confidence')
  wrapper.unmount()
})

it('shows numbered detection markers and highlights the selected finding', async () => {
  const wrapper = mount(PlaygroundVisualResult, {
    props: {
      image: 'source',
      result: {
        model: 'detector',
        data: [
          { label: 'person', confidence: 0.9, box: { x: 0.5, y: 0.5, width: 0.4, height: 0.6 } },
          { label: 'chair', confidence: 0.8, box: { x: 0.2, y: 0.5, width: 0.2, height: 0.4 } },
        ],
      },
    },
  })
  expect(wrapper.get('[data-marker="1"]').text()).toBe('1')
  expect(wrapper.get('[data-marker="2"]').text()).toBe('2')
  await wrapper.get('[aria-label="Finding 1: person"]').trigger('click')
  expect(wrapper.get('[data-marker="2"]').attributes('style')).toContain('opacity: 0.35')
  expect(wrapper.text()).toContain('90% confidence')
  wrapper.unmount()
})

it('shows instance masks and oriented polygons using Infer image dimensions', () => {
  const wrapper = mount(PlaygroundVisualResult, {
    props: {
      image: 'source',
      result: {
        model: 'specialized',
        usage: { image_width: 200, image_height: 100 },
        data: [
          {
            label: 'boat',
            confidence: 0.9,
            mask: 'data:image/png;base64,AA==',
            oriented_box: {
              corners_pixels: [
                [0, 0],
                [200, 0],
                [200, 100],
                [0, 100],
              ],
              angle_radians: 0,
            },
          },
        ],
      },
    },
  })
  expect(wrapper.get('polygon').attributes('points')).toBe('0,0 1000,0 1000,1000 0,1000')
  expect(wrapper.html()).toContain('data:image/png;base64,AA==')
  wrapper.unmount()
})