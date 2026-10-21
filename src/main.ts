import { createApp } from 'vue'

import 'unfonts.css'

import App from './App.vue'
import ControlOverlay from './components/ControlOverlay.vue'
import FlightPlayer from './components/FlightPlayer.vue'
import './style.css'

if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installTauriMock } = await import('./dev/mockTauri')
  installTauriMock()
}

const view = new URLSearchParams(location.search).get('view')
const overlay = view === 'overlay'
if (overlay) document.documentElement.classList.add('overlay-view')

if (view === 'flight') document.documentElement.classList.add('flight-view')

createApp(overlay ? ControlOverlay : view === 'flight' ? FlightPlayer : App).mount('#app')