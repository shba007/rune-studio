import { createApp } from 'vue'

import 'unfonts.css'

import App from './App.vue'
import './style.css'

// In a plain browser (no Tauri) a dev build answers the backend commands with fixtures.
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installTauriMock } = await import('./dev/mockTauri')
  installTauriMock()
}

createApp(App).mount('#app')