import { createRouter, createWebHistory } from 'vue-router'
import StudioView from '@/views/StudioView.vue'
import MediaComparatorView from '@/views/MediaComparatorView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      redirect: '/studio',
    },
    {
      path: '/studio',
      name: 'studio',
      component: StudioView,
    },
    {
      path: '/media-comparator',
      name: 'media-comparator',
      component: MediaComparatorView,
    },
  ],
})

export default router