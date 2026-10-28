import '../site.css';
import { assessSnapshot } from '../unit/checks.js';

const button = document.querySelector('#run-checks');
const status = document.querySelector('#run-status');
const results = document.querySelector('#results');
const endpoints = {
  health: '/api/health',
  providers: '/api/providers',
  quota: '/api/quota',
  stats: '/api/usage/stats?period=all',
};

async function requestJson(url) {
  try {
    const response = await fetch(url, { headers: { Accept: 'application/json' } });
    return { status: response.status, body: await response.json() };
  } catch {
    return { status: 0, body: null };
  }
}

button.addEventListener('click', async () => {
  button.disabled = true;
  status.textContent = 'Running read-only API checks…';
  results.replaceChildren();
  try {
    const snapshot = Object.fromEntries(await Promise.all(
      Object.entries(endpoints).map(async ([name, url]) => [name, await requestJson(url)]),
    ));
    const checks = assessSnapshot(snapshot);
    for (const check of checks) {
      const item = document.createElement('li');
      item.className = check.passed ? 'pass' : 'fail';
      item.textContent = `${check.passed ? 'PASS' : 'FAIL'} · ${check.name}: ${check.detail}`;
      results.append(item);
    }
    const passed = checks.filter(check => check.passed).length;
    status.textContent = checks.every(check => check.passed)
      ? `${passed} of ${checks.length} checks passed.`
      : `${passed} of ${checks.length} checks passed; review failures below.`;
  } finally {
    button.disabled = false;
  }
});
