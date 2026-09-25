import { describe, it, expect } from 'vite-plus/test'
import { mount } from '@vue/test-utils'
import App from '../App.vue'
import router from '../router'

describe('App Shell', () => {
  it('mounts and renders workflow navigation links', async () => {
    await router.push('/')
    await router.isReady()

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    })

    expect(wrapper.text()).toContain('3D Studio')
    expect(wrapper.text()).toContain('Media Comparator')
  })
})