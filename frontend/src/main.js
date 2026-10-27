import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

const target = document.getElementById('app') || document.getElementById('dashboard-root') || document.body
if (target) {
  target.innerHTML = ''
}
const app = mount(App, { target })

window.NineRouterDashboard = {
  app,
  mount(rootId) {
    const el = document.getElementById(rootId)
    if (el && !el.hasChildNodes()) {
      return mount(App, { target: el })
    }
  }
}

export default app
