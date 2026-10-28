const credentialKeys = new Set([
  'accesstoken', 'refreshtoken', 'idtoken', 'clientsecret', 'apikey', 'authorization', 'code',
]);

function containsCredentials(value) {
  if (Array.isArray(value)) return value.some(containsCredentials);
  if (!value || typeof value !== 'object') return false;
  return Object.entries(value).some(([key, child]) => {
    const normalized = key.toLowerCase().replace(/[^a-z0-9]/g, '');
    return credentialKeys.has(normalized) || containsCredentials(child);
  });
}

function validTotal(value) {
  return value === null || (typeof value === 'number' && Number.isFinite(value) && value >= 0);
}

export function assessSnapshot(snapshot) {
  const checks = [];
  const add = (name, passed, detail) => checks.push({ name, passed, detail });
  const health = snapshot.health;
  add('Rust health', health.status === 200 && health.body?.status === 'ok' && health.body?.engine === 'native-rust',
    health.status === 200 ? 'Health endpoint responded.' : `Health endpoint returned HTTP ${health.status || 'unavailable'}.`);

  const providerResponse = snapshot.providers;
  const connections = providerResponse.body?.connections;
  const providersValid = providerResponse.status === 200 && Array.isArray(connections);
  add('Provider list', providersValid, providersValid ? `${connections.length} saved connection(s).` : 'Provider list response is unavailable or malformed.');

  const quotaResponse = snapshot.quota;
  const quotaRows = quotaResponse.body?.data;
  const quotaShapeValid = quotaResponse.status === 200 && quotaResponse.body?.object === 'list' && Array.isArray(quotaRows);
  const connectionIds = new Set((providersValid ? connections : []).map(item => item.id));
  const quotaMatches = quotaShapeValid && providersValid && quotaRows.length === connections.length
    && quotaRows.every(item => typeof item.id === 'string' && connectionIds.has(item.id));
  add('Quota list matches connections', quotaMatches, quotaMatches ? `${quotaRows.length} quota item(s).` : 'Quota list is malformed or contains unrelated connections.');

  const redacted = providersValid && quotaShapeValid
    && !containsCredentials(providerResponse.body) && !containsCredentials(quotaResponse.body);
  add('Credential redaction', redacted, redacted ? 'No credential fields in provider or quota responses.' : 'Credential fields were present or a response was unavailable.');

  const statsResponse = snapshot.stats;
  const stats = statsResponse.body;
  const statsValid = statsResponse.status === 200 && stats?.period === 'all'
    && Number.isInteger(stats.totalRequests) && stats.totalRequests >= 0
    && validTotal(stats.totalPromptTokens) && validTotal(stats.totalCompletionTokens)
    && validTotal(stats.totalTokens) && validTotal(stats.totalCost);
  add('Persisted usage stats', statsValid, statsValid ? `${stats.totalRequests} recorded request(s).` : 'Usage totals are unavailable, malformed, or negative.');
  return checks;
}
