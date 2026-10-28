import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import puppeteer from 'puppeteer-core';

const base = new URL(process.env.ROUTER_URL ?? 'http://127.0.0.1:20129');
const provider = process.env.OAUTH_PROVIDER ?? 'antigravity';
const chromePaths = [
  process.env.CHROME_PATH,
  process.env.PROGRAMFILES && path.join(process.env.PROGRAMFILES, 'Google/Chrome/Application/chrome.exe'),
  process.env['PROGRAMFILES(X86)'] && path.join(process.env['PROGRAMFILES(X86)'], 'Google/Chrome/Application/chrome.exe'),
  process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Google/Chrome/Application/chrome.exe'),
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
].filter(Boolean);
const executablePath = chromePaths.find(existsSync);
assert(executablePath, 'Set CHROME_PATH to an installed Chrome executable.');
assert(['127.0.0.1', 'localhost', '[::1]'].includes(base.hostname), 'Use a loopback ROUTER_URL.');
assert(['antigravity', 'codex'].includes(provider), 'OAUTH_PROVIDER must be antigravity or codex.');

const profile = await mkdtemp(path.join(os.tmpdir(), '9router-oauth-smoke-'));
const sensitiveFields = new Set(['accesstoken', 'refreshtoken', 'idtoken', 'clientsecret', 'apikey', 'authorization', 'code']);
function assertNoCredentials(value) {
  if (Array.isArray(value)) return value.forEach(assertNoCredentials);
  if (!value || typeof value !== 'object') return;
  for (const [key, child] of Object.entries(value)) {
    const normalized = key.toLowerCase().replace(/[^a-z0-9]/g, '');
    assert(!sensitiveFields.has(normalized), `Credential field exposed: ${key}`);
    assertNoCredentials(child);
  }
}
async function getJson(url) {
  const response = await fetch(new URL(url, base));
  const body = await response.json();
  assert(response.ok, `GET ${url} returned HTTP ${response.status}.`);
  assertNoCredentials(body);
  return { response, body };
}

let browser;
try {
  browser = await puppeteer.launch({ executablePath, headless: false, userDataDir: profile, defaultViewport: null, args: ['--window-size=1280,900'] });
  const page = await browser.newPage();
  await page.goto(base.href, { waitUntil: 'domcontentloaded' });
  const health = await getJson('/api/health');
  assert.equal(health.body.status, 'ok', 'Rust dashboard health check failed.');
  const before = await getJson('/api/providers');
  assert.equal(before.body.connections.length, 0, 'Run only against an empty Rust database; this test creates a real OAuth connection.');

  const start = await page.evaluate(async id => {
    const response = await fetch(`/api/oauth/${id}/start`, { method: 'POST' });
    return { status: response.status, body: await response.json() };
  }, provider);
  assert.equal(start.status, 200, `OAuth start returned HTTP ${start.status}: ${start.body.error ?? 'unknown error'}.`);
  const authUrl = new URL(start.body.authUrl);
  assert.equal(authUrl.host, provider === 'antigravity' ? 'accounts.google.com' : 'auth.openai.com');
  const callbackUrl = new URL(authUrl.searchParams.get('redirect_uri'));
  assert.equal(callbackUrl.origin, base.origin, 'Dashboard and callback must use the same loopback origin.');
  assert.equal(callbackUrl.pathname, '/api/oauth/callback');

  console.log(`Chrome opened for ${provider}. Complete provider sign-in and consent; this script never reads or logs credentials.`);
  await page.goto(authUrl.href, { waitUntil: 'domcontentloaded' });
  const callbackHandle = await page.waitForFunction(origin => {
    if (location.origin !== origin || location.pathname !== '/api/oauth/callback') return false;
    const text = document.body.innerText;
    return text.includes('Account connected.') ? 'success' : text.includes('Sign-in failed.') ? 'failure' : false;
  }, { timeout: 5 * 60_000 }, base.origin);
  const callbackResult = await callbackHandle.jsonValue();
  await callbackHandle.dispose();
  assert.equal(callbackResult, 'success', 'Provider callback did not save an account.');

  const providers = await getJson('/api/providers');
  const connection = providers.body.connections.find(item => item.provider === provider);
  assert(connection?.id && connection.hasOAuthToken, 'OAuth callback did not create a token-backed connection.');
  const quotas = await getJson('/api/quota');
  const quota = quotas.body.data.find(item => item.id === connection.id);
  assert(quota, 'Quota list omitted the new connection.');
  const usage = await getJson(`/api/usage/${encodeURIComponent(connection.id)}`);
  assert.equal(usage.body.connectionId, connection.id, 'Per-connection quota response did not match the saved connection.');

  await page.goto(new URL('/#quota', base).href, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(name => document.body.innerText.includes(name), { timeout: 30_000 }, provider === 'antigravity' ? 'Antigravity' : 'OpenAI Codex');
  console.log(JSON.stringify({ provider, callback: 'success', providerListRedacted: true, quotaHttp: quotas.response.status, connectionUsageHttp: usage.response.status, quotaState: quota.message ? 'unavailable' : 'returned', dashboard: 'visible', connectionRemainsSaved: true }));
} catch (error) {
  console.error(`OAuth browser smoke failed: ${error?.message ?? 'unknown error'}`);
  process.exitCode = 1;
} finally {
  await browser?.close();
  await rm(profile, { recursive: true, force: true });
}
