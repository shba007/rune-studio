import { createApp } from 'vue'

import 'unfonts.css'

import App from './App.vue'
import ControlOverlay from './components/ControlOverlay.vue'
import './style.css'

// In a plain browser (no Tauri) a dev build answers the backend commands with fixtures.
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installTauriMock } = await import('./dev/mockTauri')
  installTauriMock()
}

// The same bundle serves the pill and the full-screen control overlay.
const overlay = new URLSearchParams(location.search).get('view') === 'overlay'

createApp(overlay ? ControlOverlay : App).mount('#app')