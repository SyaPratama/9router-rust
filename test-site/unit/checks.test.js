import test from 'node:test';
import assert from 'node:assert/strict';
import { assessSnapshot } from './checks.js';

const emptySnapshot = () => ({
  health: { status: 200, body: { status: 'ok', engine: 'native-rust' } },
  providers: { status: 200, body: { connections: [] } },
  quota: { status: 200, body: { object: 'list', data: [] } },
  stats: { status: 200, body: { period: 'all', totalRequests: 0, totalPromptTokens: 0, totalCompletionTokens: 0, totalTokens: 0, totalCost: 0 } },
});

test('empty Rust database passes read-only API checks', () => {
  assert(assessSnapshot(emptySnapshot()).every(check => check.passed));
});

test('honest per-connection unavailable quota remains valid', () => {
  const snapshot = emptySnapshot();
  snapshot.providers.body.connections.push({ id: 'conn-1', provider: 'antigravity', hasOAuthToken: true });
  snapshot.quota.body.data.push({ id: 'conn-1', provider: 'antigravity', quotas: {}, message: 'Provider unavailable.' });
  const checks = assessSnapshot(snapshot);
  assert(checks.find(check => check.name === 'Quota list matches connections').passed);
  assert(checks.find(check => check.name === 'Credential redaction').passed);
});

test('quota rows for unsaved connections fail', () => {
  const snapshot = emptySnapshot();
  snapshot.quota.body.data.push({ id: 'not-saved', provider: 'antigravity', quotas: {} });
  assert.equal(assessSnapshot(snapshot).find(check => check.name === 'Quota list matches connections').passed, false);
});

test('nested OAuth secrets fail credential-redaction check', () => {
  const snapshot = emptySnapshot();
  snapshot.providers.body.connections.push({ id: 'conn-1', data: { nested: { access_token: 'must-not-escape' } } });
  snapshot.quota.body.data.push({ id: 'conn-1', provider: 'antigravity', quotas: {} });
  assert.equal(assessSnapshot(snapshot).find(check => check.name === 'Credential redaction').passed, false);
});

test('negative usage totals fail validation', () => {
  const snapshot = emptySnapshot();
  snapshot.stats.body.totalRequests = -1;
  assert.equal(assessSnapshot(snapshot).find(check => check.name === 'Persisted usage stats').passed, false);
});
