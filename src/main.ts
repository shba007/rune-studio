import { createApp } from 'vue'

import 'unfonts.css'

import App from './App.vue'
import ControlOverlay from './components/ControlOverlay.vue'
import FlightPlayer from './components/FlightPlayer.vue'
import './style.css'

// In a plain browser (no Tauri) a dev build answers the backend commands with fixtures.
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installTauriMock } = await import('./dev/mockTauri')
  installTauriMock()
}

// The same bundle serves the pill and the full-screen overlay (control and screen analysis).
const view = new URLSearchParams(location.search).get('view')
const overlay = view === 'overlay'
// The full-screen overlay never shows the Vue devtools button (see style.css).
if (overlay) document.documentElement.classList.add('overlay-view')

// The Flight Player is an ordinary opaque window (the pill's page is transparent).
if (view === 'flight') document.documentElement.classList.add('flight-view')

createApp(overlay ? ControlOverlay : view === 'flight' ? FlightPlayer : App).mount('#app')