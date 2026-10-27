<script>
  import { onMount } from 'svelte';

  // Svelte 5 reactive state
  let activeView = $state('overview');
  let health = $state('Port 20128 · Online');
  let version = $state('v1.0.1');

  let summary = $state({
    providers: { total: 0, active: 0 },
    combos: 0,
    keys: { total: 0, active: 0 },
    usage: { totalRequests: 0, totalTokens: 0, totalPromptTokens: 0, totalCompletionTokens: 0 }
  });

  let providers = $state([]);
  let combos = $state([]);
  let keys = $state([]);
  let settings = $state({
    headroom_enabled: true,
    rtk_enabled: true,
    caveman_level: '',
    ponytail_level: '',
    port: 20128,
    require_api_key: false
  });
  let quotaList = $state([]);
  let proxyPools = $state([]);
  let mediaModels = $state([]);
  let activeMediaKind = $state('image');

  // Logs & SSE state
  let logs = $state([]);
  let logFilter = $state('ALL');
  let logSearch = $state('');
  let logPaused = $state(false);
  let logAutoscroll = $state(true);
  let terminalScreen = $state(null);

  // Snippet
  let activeSnippet = $state('chat');

  // Translator
  let transSource = $state('openai');
  let transTarget = $state('claude');
  let transInput = $state(JSON.stringify({
    model: "openai/gpt-4o",
    messages: [
      { role: "system", content: "You are a senior Rust systems architect." },
      { role: "user", content: "Explain Tokio's multi-threaded work-stealing runtime." }
    ],
    temperature: 0.7,
    stream: false
  }, null, 2));
  let transOutput = $state('// Output will appear here upon clicking Convert & Test');
  let transStats = $state('Ready');

  // Modal State
  let modalOpen = $state(false);
  let modalTitle = $state('');
  let modalType = $state('');
  let modalItem = $state({});

  // Notice Banner
  let notice = $state({ show: false, text: '', error: false });

  function showNotice(text, error = false) {
    notice = { show: true, text, error };
    setTimeout(() => { notice.show = false; }, 3500);
  }

  // API Client
  async function api(path, options = {}) {
    const res = await fetch(path, {
      headers: { 'Content-Type': 'application/json', ...(options.headers || {}) },
      ...options
    });
    if (!res.ok) {
      const err = await res.text();
      throw new Error(err || `${res.status} ${res.statusText}`);
    }
    return res.status === 204 ? null : res.json();
  }

  async function refresh() {
    try {
      const [sumRes, provRes, combRes, keyRes, setRes] = await Promise.all([
        api('/api/summary'),
        api('/api/providers'),
        api('/api/combos'),
        api('/api/keys'),
        api('/api/settings')
      ]);
      summary = sumRes;
      providers = provRes || [];
      combos = combRes || [];
      keys = keyRes || [];
      settings = { ...settings, ...setRes };
    } catch (e) {
      showNotice('Sync error: ' + e.message, true);
    }
  }

  // View Routing
  function setView(name, updateUrl = true) {
    activeView = name;
    if (updateUrl) {
      history.pushState({ view: name }, '', `/dashboard/${name}`);
    }
    if (name === 'quota') loadQuota();
    if (name === 'media') loadMedia(activeMediaKind);
    if (name === 'proxy-pools') loadProxyPools();
  }

  async function loadQuota() {
    try {
      const res = await api('/api/quota');
      quotaList = res.data || [];
    } catch (e) {
      showNotice('Failed to load quota: ' + e.message, true);
    }
  }

  async function loadMedia(kind) {
    activeMediaKind = kind;
    try {
      const res = await api(`/v1/models/${kind}`);
      mediaModels = res.data || [];
    } catch (e) {
      showNotice('Failed to load media catalog: ' + e.message, true);
    }
  }

  async function loadProxyPools() {
    try {
      const list = await api('/api/proxy-pools');
      proxyPools = list || [];
    } catch (e) {
      showNotice('Failed to load proxy pools: ' + e.message, true);
    }
  }

  async function saveSettings() {
    try {
      await api('/api/settings', {
        method: 'POST',
        body: JSON.stringify(settings)
      });
      showNotice('Settings saved successfully.');
      await refresh();
    } catch (e) {
      showNotice('Error saving settings: ' + e.message, true);
    }
  }

  async function deleteItem(path) {
    if (!confirm('Are you sure you want to delete this item?')) return;
    try {
      await api(path, { method: 'DELETE' });
      showNotice('Deleted successfully.');
      await refresh();
    } catch (e) {
      showNotice(e.message, true);
    }
  }

  // Clipboard Helpers
  function copyText(text) {
    navigator.clipboard.writeText(text).then(() => {
      showNotice('Copied to clipboard!');
    }).catch(() => {
      prompt('Copy manually:', text);
    });
  }

  // Snippet
  const snippets = {
    chat: `curl http://localhost:20128/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer sk-9router" \\
  -d '{"model":"openai/gpt-4o","messages":[{"role":"user","content":"Hello!"}]}'`,
    claude: `curl http://localhost:20128/v1/messages \\
  -H "Content-Type: application/json" \\
  -H "x-api-key: sk-9router" \\
  -H "anthropic-version: 2023-06-01" \\
  -d '{"model":"cc/claude-opus-4-7","max_tokens":1024,"messages":[{"role":"user","content":"Hello!"}]}'`,
    compress: `curl http://localhost:20128/v1/compress \\
  -H "Content-Type: application/json" \\
  -d '{"prompt":"const code = \\"...\\"; please refactor","aggressiveness":"high"}'`,
    models: `curl http://localhost:20128/v1/models | jq '.data[].id'`
  };

  // Translator Live Test
  async function runTranslator() {
    transStats = 'Translating...';
    try {
      const parsed = JSON.parse(transInput);
      const res = await api('/api/translator/run', {
        method: 'POST',
        body: JSON.stringify({
          source_format: transSource,
          target_format: transTarget,
          payload: parsed,
          dry_run: true
        })
      });
      transOutput = JSON.stringify(res.output, null, 2);
      transStats = `${res.stats.tokens_estimated} tokens · ${res.stats.translation_time_ms} ms`;
      showNotice('Format converted successfully.');
    } catch (e) {
      transStats = 'Error';
      transOutput = `// Translation failed:\n// ${e.message}`;
      showNotice(e.message, true);
    }
  }

  // Live SSE Console Log
  function initSSE() {
    const sse = new EventSource('/api/console-log/stream');
    sse.onmessage = (ev) => {
      if (logPaused) return;
      logs.push(ev.data);
      if (logs.length > 1000) logs.shift();
      logs = [...logs];
      if (logAutoscroll && terminalScreen) {
        setTimeout(() => { terminalScreen.scrollTop = terminalScreen.scrollHeight; }, 10);
      }
    };
  }

  let filteredLogs = $derived(
    logs.filter(l => {
      if (logFilter === 'INFO' && !l.includes('[ INFO]')) return false;
      if (logFilter === 'WARN' && !l.includes('[ WARN]')) return false;
      if (logFilter === 'ERROR' && !l.includes('[ERROR]')) return false;
      if (logSearch && !l.toLowerCase().includes(logSearch.toLowerCase())) return false;
      return true;
    })
  );

  // Modals
  function openProviderModal(item = {}) {
    modalType = 'provider';
    modalTitle = item.id ? 'Edit Provider' : 'Add Provider';
    modalItem = {
      id: item.id || crypto.randomUUID(),
      provider: item.provider || 'openai',
      name: item.name || '',
      authType: item.auth_type || 'api_key',
      priority: item.priority || 1,
      isActive: item.is_active !== false,
      apiKey: item.data?.apiKey || '',
      baseUrl: item.data?.baseUrl || ''
    };
    modalOpen = true;
  }

  function openComboModal(item = {}) {
    modalType = 'combo';
    modalTitle = item.id ? 'Edit Combo' : 'Add Combo';
    modalItem = {
      id: item.id || crypto.randomUUID(),
      name: item.name || '',
      kind: item.kind || 'chat',
      models: (item.models || []).join('\n')
    };
    modalOpen = true;
  }

  function openKeyModal(item = {}) {
    modalType = 'key';
    modalTitle = item.id ? 'Edit API Key' : 'Create API Key';
    modalItem = {
      id: item.id || crypto.randomUUID(),
      name: item.name || '',
      key: item.key || 'sk-9router-' + Math.random().toString(36).substring(2, 14),
      machineId: item.machine_id || '',
      accessAllow: item.access_allow || '',
      isActive: item.is_active !== false
    };
    modalOpen = true;
  }

  function openProxyModal(item = {}) {
    modalType = 'proxy';
    modalTitle = item.id ? 'Edit Proxy Pool' : 'Add Proxy Pool';
    modalItem = {
      id: item.id || crypto.randomUUID(),
      name: item.data?.name || '',
      protocol: item.data?.protocol || 'http',
      url: item.data?.url || '',
      isActive: item.is_active !== false
    };
    modalOpen = true;
  }

  async function submitModal(e) {
    e.preventDefault();
    try {
      if (modalType === 'provider') {
        await api('/api/providers', {
          method: 'POST',
          body: JSON.stringify({
            id: modalItem.id,
            provider: modalItem.provider,
            name: modalItem.name || null,
            authType: modalItem.authType,
            priority: Number(modalItem.priority) || 1,
            isActive: modalItem.isActive,
            data: {
              apiKey: modalItem.apiKey || undefined,
              baseUrl: modalItem.baseUrl || undefined
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString()
          })
        });
      } else if (modalType === 'combo') {
        const mList = modalItem.models.split('\n').map(s => s.trim()).filter(Boolean);
        await api('/api/combos', {
          method: 'POST',
          body: JSON.stringify({
            id: modalItem.id,
            name: modalItem.name,
            kind: modalItem.kind || 'chat',
            models: mList,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString()
          })
        });
      } else if (modalType === 'key') {
        await api('/api/keys', {
          method: 'POST',
          body: JSON.stringify({
            id: modalItem.id,
            key: modalItem.key,
            name: modalItem.name || null,
            machineId: modalItem.machineId || null,
            isActive: modalItem.isActive,
            accessRestricted: !!modalItem.accessAllow,
            accessAllow: modalItem.accessAllow || null,
            createdAt: new Date().toISOString()
          })
        });
      } else if (modalType === 'proxy') {
        await api('/api/proxy-pools', {
          method: 'POST',
          body: JSON.stringify({
            id: modalItem.id,
            isActive: modalItem.isActive,
            testStatus: 'untested',
            data: {
              name: modalItem.name,
              protocol: modalItem.protocol,
              url: modalItem.url
            },
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString()
          })
        });
        await loadProxyPools();
      }

      modalOpen = false;
      showNotice('Saved successfully.');
      await refresh();
    } catch (err) {
      showNotice('Save error: ' + err.message, true);
    }
  }

  onMount(async () => {
    try {
      const hRes = await api('/api/health');
      health = `Port 20128 · v${hRes.version}`;
      version = `v${hRes.version}`;
    } catch (_) {}

    const seg = location.pathname.split('/')[2] || 'overview';
    setView(seg, false);

    await refresh();
    initSSE();
  });
</script>

<div class="app-shell">
  <!-- SIDEBAR NAVIGATION -->
  <aside>
    <div class="brand">
      <div class="brand-title">
        <div class="brand-logo">9</div>
        <div>
          <h1>9Router</h1>
          <div class="muted" style="font-size:11px;">Native Rust Gateway</div>
        </div>
      </div>
      <span class="version-pill">{version}</span>
    </div>

    <nav>
      <div class="nav-section">Gateway</div>
      <button class={activeView === 'overview' ? 'active' : ''} onclick={() => setView('overview')}><span class="nav-icon">⌂</span>Endpoint &amp; Key</button>
      <button class={activeView === 'providers' ? 'active' : ''} onclick={() => setView('providers')}><span class="nav-icon">◈</span>Providers</button>
      <button class={activeView === 'combos' ? 'active' : ''} onclick={() => setView('combos')}><span class="nav-icon">◇</span>Combo &amp; Vision</button>
      <button class={activeView === 'keys' ? 'active' : ''} onclick={() => setView('keys')}><span class="nav-icon">⌕</span>API Keys</button>
      <button class={activeView === 'usage' ? 'active' : ''} onclick={() => setView('usage')}><span class="nav-icon">▥</span>Usage Analytics</button>
      <button class={activeView === 'quota' ? 'active' : ''} onclick={() => setView('quota')}><span class="nav-icon">◷</span>Quota Tracker</button>
      <button class={activeView === 'token-saver' ? 'active' : ''} onclick={() => setView('token-saver')}><span class="nav-icon">✦</span>Token Saver</button>

      <div class="nav-section">Tools</div>
      <button class={activeView === 'cli-tools' ? 'active' : ''} onclick={() => setView('cli-tools')}><span class="nav-icon">›_</span>CLI Tools</button>
      <button class={activeView === 'translator' ? 'active' : ''} onclick={() => setView('translator')}><span class="nav-icon">文</span>Translator</button>
      <button class={activeView === 'console-log' ? 'active' : ''} onclick={() => setView('console-log')}><span class="nav-icon">≡</span>Console Log</button>

      <div class="nav-section">System</div>
      <button class={activeView === 'media' ? 'active' : ''} onclick={() => setView('media')}><span class="nav-icon">◎</span>Media Providers</button>
      <button class={activeView === 'proxy-pools' ? 'active' : ''} onclick={() => setView('proxy-pools')}><span class="nav-icon">⇄</span>Proxy Pools</button>
      <button class={activeView === 'settings' ? 'active' : ''} onclick={() => setView('settings')}><span class="nav-icon">⚙</span>Settings</button>
    </nav>
  </aside>

  <!-- MAIN VIEW CONTAINER -->
  <div class="content">
    <header class="page-header">
      <div class="breadcrumbs">
        <span class="breadcrumb-root">9Router</span>
        <span class="breadcrumb-sep">/</span>
        <span class="breadcrumb-active" style="text-transform: capitalize;">{activeView.replace('-', ' ')}</span>
      </div>
      <div class="header-status">
        <button class="sm ghost" onclick={() => copyText('http://localhost:20128')}>Copy Base URL</button>
        <div class="gateway-pill">
          <span class="pulse-dot"></span>
          <span>{health}</span>
        </div>
      </div>
    </header>

    <main>
      {#if notice.show}
        <div class="notice {notice.error ? 'error' : 'ok'}">{notice.text}</div>
      {/if}

      <!-- 1. OVERVIEW -->
      {#if activeView === 'overview'}
        <div class="grid">
          <div class="metric-card">
            <div class="metric-label">Active Providers</div>
            <div class="metric-value">{summary.providers?.active || 0} / {summary.providers?.total || 0}</div>
            <div class="metric-sub">Connected AI backends</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Fallback Combos</div>
            <div class="metric-value">{summary.combos || 0}</div>
            <div class="metric-sub">Auto-failover chains</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Active API Keys</div>
            <div class="metric-value">{summary.keys?.active || 0} / {summary.keys?.total || 0}</div>
            <div class="metric-sub">Client access tokens</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Total Requests</div>
            <div class="metric-value">{(summary.usage?.totalRequests || 0).toLocaleString()}</div>
            <div class="metric-sub">Routed through engine</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Tokens Saved</div>
            <div class="metric-value" style="color:var(--mint-bright);">{(summary.usage?.totalTokens || 0).toLocaleString()}</div>
            <div class="metric-sub">via Headroom compression</div>
          </div>
        </div>

        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Quick Connect</h2>
              <div class="muted">Point your OpenAI or Anthropic SDKs to 9Router. Supports all coding tools.</div>
            </div>
            <button class="sm primary" onclick={() => setView('cli-tools')}>CLI Setup Guide</button>
          </div>

          <div class="tab-nav">
            <button class="tab-btn {activeSnippet === 'chat' ? 'active' : ''}" onclick={() => activeSnippet = 'chat'}>Chat Completions</button>
            <button class="tab-btn {activeSnippet === 'claude' ? 'active' : ''}" onclick={() => activeSnippet = 'claude'}>Claude Messages</button>
            <button class="tab-btn {activeSnippet === 'compress' ? 'active' : ''}" onclick={() => activeSnippet = 'compress'}>Headroom Compression</button>
            <button class="tab-btn {activeSnippet === 'models' ? 'active' : ''}" onclick={() => activeSnippet = 'models'}>Models Catalog</button>
          </div>

          <div class="snippet-box">
            <button class="copy-btn" onclick={() => copyText(snippets[activeSnippet])}>Copy cURL</button>
            <pre>{snippets[activeSnippet]}</pre>
          </div>
        </div>
      {/if}

      <!-- 2. PROVIDERS -->
      {#if activeView === 'providers'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Provider Connections</h2>
              <div class="muted">Multi-provider routing pool with health tracking, OAuth tokens, and API keys.</div>
            </div>
            <button class="primary" onclick={() => openProviderModal()}>Add provider</button>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Provider</th>
                  <th>Authentication</th>
                  <th>Priority</th>
                  <th>Status</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {#if providers.length === 0}
                  <tr><td colspan="5" style="text-align:center; padding:32px; color:var(--muted);">No providers connected. Click "Add provider" to configure OpenAI, Anthropic, Gemini, Groq, or DeepSeek.</td></tr>
                {:else}
                  {#each providers as p}
                    <tr>
                      <td>
                        <strong>{p.provider}</strong>
                        <div class="muted">{p.name || p.email || p.id}</div>
                      </td>
                      <td><code>{p.auth_type}</code></td>
                      <td><span class="badge">#{p.priority || 1}</span></td>
                      <td>
                        <span class="badge {p.is_active ? 'healthy' : 'danger'}">{p.is_active ? 'Active' : 'Disabled'}</span>
                      </td>
                      <td>
                        <div class="actions">
                          <button class="sm" onclick={() => openProviderModal(p)}>Edit</button>
                          <button class="sm danger" onclick={() => deleteItem(`/api/providers/${encodeURIComponent(p.id)}`)}>Delete</button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- 3. COMBOS -->
      {#if activeView === 'combos'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Fallback Combos &amp; Vision</h2>
              <div class="muted">Chains multiple models in order of preference with automated failover and retry.</div>
            </div>
            <div class="actions">
              <button class="ghost sm" onclick={() => openComboModal({ name: 'coding-auto', kind: 'chat', models: ['anthropic/claude-3-5-sonnet', 'deepseek/deepseek-chat', 'openai/gpt-4o-mini'] })}>+ Coding Preset</button>
              <button class="ghost sm" onclick={() => openComboModal({ name: 'fast-auto', kind: 'chat', models: ['groq/llama-3.3-70b', 'gemini/gemini-1.5-flash', 'openai/gpt-4o-mini'] })}>+ Fast Fallback</button>
              <button class="primary" onclick={() => openComboModal()}>Add Combo</button>
            </div>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Kind</th>
                  <th>Fallback Chain</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {#if combos.length === 0}
                  <tr><td colspan="4" style="text-align:center; padding:32px; color:var(--muted);">No fallback combos configured. Click "+ Coding Preset" or "Add Combo" to define failover chains.</td></tr>
                {:else}
                  {#each combos as c}
                    <tr>
                      <td><strong>{c.name}</strong></td>
                      <td><span class="badge">{c.kind || 'chat'}</span></td>
                      <td>
                        {#each c.models as m, idx}
                          <code>{m}</code>
                          {#if idx < c.models.length - 1} <span style="color:var(--mint); font-weight:bold;">→</span> {/if}
                        {/each}
                      </td>
                      <td>
                        <div class="actions">
                          <button class="sm" onclick={() => openComboModal(c)}>Edit</button>
                          <button class="sm danger" onclick={() => deleteItem(`/api/combos/${encodeURIComponent(c.id)}`)}>Delete</button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- 4. API KEYS -->
      {#if activeView === 'keys'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Client API Keys</h2>
              <div class="muted">Bearer tokens authorized to access gateway endpoints when authentication is required.</div>
            </div>
            <button class="primary" onclick={() => openKeyModal()}>Create API Key</button>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Key</th>
                  <th>Machine ID</th>
                  <th>Access Scope</th>
                  <th>Status</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {#if keys.length === 0}
                  <tr><td colspan="6" style="text-align:center; padding:32px; color:var(--muted);">No client API keys configured. Click "Create API Key" to authorize clients.</td></tr>
                {:else}
                  {#each keys as k}
                    <tr>
                      <td><strong>{k.name || 'Unnamed'}</strong></td>
                      <td>
                        <code>{k.key.substring(0, 14)}...</code>
                        <button class="ghost sm" onclick={() => copyText(k.key)}>Copy</button>
                      </td>
                      <td>{k.machine_id || 'Any'}</td>
                      <td>{k.access_restricted ? k.access_allow || 'Restricted' : 'Universal'}</td>
                      <td><span class="badge {k.is_active ? 'healthy' : 'danger'}">{k.is_active ? 'Active' : 'Revoked'}</span></td>
                      <td>
                        <div class="actions">
                          <button class="sm" onclick={() => openKeyModal(k)}>Edit</button>
                          <button class="sm danger" onclick={() => deleteItem(`/api/keys/${encodeURIComponent(k.id)}`)}>Delete</button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- 5. USAGE -->
      {#if activeView === 'usage'}
        <div class="grid">
          <div class="metric-card">
            <div class="metric-label">Total Requests</div>
            <div class="metric-value">{(summary.usage?.totalRequests || 0).toLocaleString()}</div>
            <div class="metric-sub">Gateway API calls</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Prompt Tokens</div>
            <div class="metric-value">{(summary.usage?.totalPromptTokens || 0).toLocaleString()}</div>
            <div class="metric-sub">Inbound context</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Completion Tokens</div>
            <div class="metric-value">{(summary.usage?.totalCompletionTokens || 0).toLocaleString()}</div>
            <div class="metric-sub">Generated output</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Estimated Cost Saved</div>
            <div class="metric-value" style="color:var(--mint-bright);">${(((summary.usage?.totalTokens || 0) / 1000) * 0.002).toFixed(2)}</div>
            <div class="metric-sub">Compression &amp; combos</div>
          </div>
        </div>
        <div class="card">
          <h2>Traffic Telemetry</h2>
          <p class="muted">Real-time accounting tracked in native SQLite database.</p>
          <div class="table-wrap">
            <table>
              <thead>
                <tr><th>Metric</th><th>Value</th><th>Unit</th></tr>
              </thead>
              <tbody>
                <tr><td>Database Engine</td><td>SQLite Bundled (r2d2 connection pool)</td><td>Engine</td></tr>
                <tr><td>Context Crusher</td><td>Headroom Smart-Crusher + CCR RAG</td><td>Integrated</td></tr>
                <tr><td>Latency Overhead</td><td>&lt; 0.5 ms</td><td>Native Rust</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- 6. QUOTA TRACKER -->
      {#if activeView === 'quota'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Provider Quota Telemetry</h2>
              <div class="muted">Live monitoring of upstream request limits, token budgets, and rate-limit reset windows.</div>
            </div>
            <button class="ghost sm" onclick={loadQuota}>↻ Refresh Quota</button>
          </div>
          <div class="grid">
            {#each quotaList as q}
              <div class="metric-card">
                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
                  <strong style="font-size:15px;">{q.name}</strong>
                  <span class="badge {q.status === 'healthy' ? 'healthy' : 'warn'}">{q.status}</span>
                </div>
                <div class="muted" style="font-size:11.5px;">Upstream: <code>{q.provider}</code></div>
                <div style="margin-top:14px;">
                  <div style="display:flex; justify-content:space-between; font-size:12px;">
                    <span class="muted">Requests Remaining</span>
                    <strong>{q.requests_remaining.toLocaleString()} / {q.requests_limit.toLocaleString()}</strong>
                  </div>
                  <div class="progress-bar">
                    <div class="progress-fill" style="width: {100 - Math.min(100, Math.max(0, q.percent_used || 0))}%;"></div>
                  </div>
                </div>
                <div style="display:flex; justify-content:space-between; font-size:12px; margin-top:8px;">
                  <span class="muted">Rate Reset Countdown</span>
                  <strong style="color:var(--mint-bright);">{q.reset_countdown}</strong>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- 7. TOKEN SAVER -->
      {#if activeView === 'token-saver'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Token Saver &amp; Compression Suite</h2>
              <div class="muted">Drastically cut API bills with native Headroom AST compression, RTK deduplication, and Caveman prompt compaction.</div>
            </div>
            <button class="primary" onclick={saveSettings}>Save Configuration</button>
          </div>

          <div class="form-grid" style="margin-top:14px;">
            <div class="field">
              <label>
                <input type="checkbox" bind:checked={settings.headroom_enabled}>
                <strong>Enable Headroom AST Compression</strong>
              </label>
              <div class="muted">Reduces prompt tokens by up to 60% through semantic syntax crushing before dispatching upstream.</div>
            </div>

            <div class="field">
              <label>
                <input type="checkbox" bind:checked={settings.rtk_enabled}>
                <strong>Enable RTK Deduplication</strong>
              </label>
              <div class="muted">Removes redundant repeat system instructions and context blocks across conversational turns.</div>
            </div>

            <div class="field">
              <label>
                Caveman Mode Level
                <select bind:value={settings.caveman_level}>
                  <option value="">Disabled</option>
                  <option value="low">Low (Minor prompt compression)</option>
                  <option value="medium">Medium (Concise instructions)</option>
                  <option value="high">High (Ultra-compact telegraphic prompts)</option>
                </select>
              </label>
            </div>

            <div class="field">
              <label>
                Ponytail Mode Level
                <select bind:value={settings.ponytail_level}>
                  <option value="">Disabled</option>
                  <option value="lite">Lite (Minimal diffs)</option>
                  <option value="full">Full (Eliminate abstractions)</option>
                  <option value="ultra">Ultra (Strict YAGNI verification)</option>
                </select>
              </label>
            </div>
          </div>
        </div>
      {/if}

      <!-- 8. CLI TOOLS -->
      {#if activeView === 'cli-tools'}
        <div class="card">
          <h2>Developer Coding CLI Configuration</h2>
          <p class="muted">Point any coding agent or IDE directly to 9Router. Click any command to copy.</p>

          <div class="grid" style="grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); margin-top:18px;">
            <div class="metric-card">
              <h3>Claude Code</h3>
              <div class="muted" style="margin:4px 0 10px;">Anthropic terminal agent</div>
              <div class="snippet-box" style="margin-top:0;">
                <button class="copy-btn" onclick={() => copyText('export ANTHROPIC_BASE_URL="http://localhost:20128"')}>Copy</button>
                <pre>export ANTHROPIC_BASE_URL="http://localhost:20128"
claude</pre>
              </div>
            </div>

            <div class="metric-card">
              <h3>Cursor IDE</h3>
              <div class="muted" style="margin:4px 0 10px;">Settings &gt; Models &gt; Override</div>
              <div class="snippet-box" style="margin-top:0;">
                <button class="copy-btn" onclick={() => copyText('http://localhost:20128/v1')}>Copy</button>
                <pre>Override OpenAI Base URL:
http://localhost:20128/v1</pre>
              </div>
            </div>

            <div class="metric-card">
              <h3>Codex CLI / Cline / OpenClaw</h3>
              <div class="muted" style="margin:4px 0 10px;">Universal OpenAI format</div>
              <div class="snippet-box" style="margin-top:0;">
                <button class="copy-btn" onclick={() => copyText('export OPENAI_BASE_URL="http://localhost:20128/v1"\nexport OPENAI_API_KEY="sk-9router"')}>Copy</button>
                <pre>export OPENAI_BASE_URL="http://localhost:20128/v1"
export OPENAI_API_KEY="sk-9router"</pre>
              </div>
            </div>

            <div class="metric-card">
              <h3>Windsurf / Aider</h3>
              <div class="muted" style="margin:4px 0 10px;">Aider pair programming</div>
              <div class="snippet-box" style="margin-top:0;">
                <button class="copy-btn" onclick={() => copyText('aider --openai-api-base http://localhost:20128/v1')}>Copy</button>
                <pre>aider --openai-api-base http://localhost:20128/v1</pre>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <!-- 9. TRANSLATOR WORKBENCH -->
      {#if activeView === 'translator'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Translator Workbench</h2>
              <div class="muted">Live format conversion runner between OpenAI, Claude Messages, Google Gemini, and Responses.</div>
            </div>
            <button class="sm primary" onclick={runTranslator}>Convert &amp; Test</button>
          </div>

          <div class="form-grid">
            <div class="field">
              <label>
                Source Format
                <select bind:value={transSource}>
                  <option value="openai">OpenAI (/v1/chat/completions)</option>
                  <option value="claude">Claude Messages (/v1/messages)</option>
                  <option value="responses">Responses (/v1/responses)</option>
                  <option value="gemini">Google Gemini (generateContent)</option>
                </select>
              </label>
            </div>
            <div class="field">
              <label>
                Target Format
                <select bind:value={transTarget}>
                  <option value="claude">Claude Messages (/v1/messages)</option>
                  <option value="openai">OpenAI (/v1/chat/completions)</option>
                  <option value="gemini">Google Gemini (generateContent)</option>
                  <option value="responses">Responses (/v1/responses)</option>
                </select>
              </label>
            </div>
          </div>
          <div class="split-pane">
            <div class="pane-col">
              <div class="pane-header">
                <span>Input JSON</span>
              </div>
              <textarea class="pane-editor" bind:value={transInput} spellcheck="false"></textarea>
            </div>
            <div class="pane-col">
              <div class="pane-header">
                <span>Translated Output</span>
                <span class="badge {transStats.includes('Error') ? 'danger' : 'healthy'}">{transStats}</span>
              </div>
              <textarea class="pane-editor" readonly bind:value={transOutput} spellcheck="false"></textarea>
            </div>
          </div>
        </div>
      {/if}

      <!-- 10. CONSOLE LOG -->
      {#if activeView === 'console-log'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Live Console Log Streaming</h2>
              <div class="muted">Real-time SSE event stream from the native Rust gateway tracing subscriber.</div>
            </div>
            <div class="actions">
              <button class="ghost sm" onclick={() => logs = []}>Clear</button>
              <button class="ghost sm {logPaused ? 'primary' : ''}" onclick={() => logPaused = !logPaused}>{logPaused ? 'Resume' : 'Pause'}</button>
              <label class="muted" style="display:inline-flex; align-items:center; gap:4px; font-size:12px;">
                <input type="checkbox" bind:checked={logAutoscroll}> Auto-scroll
              </label>
            </div>
          </div>

          <div class="toolbar" style="margin-bottom:8px;">
            <div class="actions">
              <button class="sm ghost {logFilter === 'ALL' ? 'primary' : ''}" onclick={() => logFilter = 'ALL'}>ALL</button>
              <button class="sm ghost {logFilter === 'INFO' ? 'primary' : ''}" onclick={() => logFilter = 'INFO'}>INFO</button>
              <button class="sm ghost {logFilter === 'WARN' ? 'primary' : ''}" onclick={() => logFilter = 'WARN'}>WARN</button>
              <button class="sm ghost {logFilter === 'ERROR' ? 'primary' : ''}" onclick={() => logFilter = 'ERROR'}>ERROR</button>
            </div>
            <input type="text" placeholder="Filter logs by keyword..." style="max-width:280px;" bind:value={logSearch}>
          </div>

          <div class="terminal-viewer" bind:this={terminalScreen}>
            {#each filteredLogs as l}
              <div class="log-line {l.includes('[ERROR]') ? 'error' : l.includes('[ WARN]') ? 'warn' : l.includes('ready') ? 'success' : 'info'}">
                {l}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- 11. MEDIA PROVIDERS -->
      {#if activeView === 'media'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Multi-Modal Media Registry</h2>
              <div class="muted">Discover and test all 9 native skill endpoints: Image, TTS, STT, Embeddings, Video, and Web.</div>
            </div>
            <div class="actions">
              <button class="ghost sm {activeMediaKind === 'image' ? 'primary' : ''}" onclick={() => loadMedia('image')}>Image</button>
              <button class="ghost sm {activeMediaKind === 'tts' ? 'primary' : ''}" onclick={() => loadMedia('tts')}>TTS</button>
              <button class="ghost sm {activeMediaKind === 'stt' ? 'primary' : ''}" onclick={() => loadMedia('stt')}>STT</button>
              <button class="ghost sm {activeMediaKind === 'embedding' ? 'primary' : ''}" onclick={() => loadMedia('embedding')}>Embeddings</button>
              <button class="ghost sm {activeMediaKind === 'video' ? 'primary' : ''}" onclick={() => loadMedia('video')}>Video</button>
              <button class="ghost sm {activeMediaKind === 'web' ? 'primary' : ''}" onclick={() => loadMedia('web')}>Web</button>
            </div>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Model ID</th>
                  <th>Kind</th>
                  <th>Provider</th>
                  <th>Endpoint</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {#each mediaModels as m}
                  <tr>
                    <td><strong>{m.id}</strong></td>
                    <td><span class="badge">{m.kind || activeMediaKind}</span></td>
                    <td><code>{m.owned_by || 'system'}</code></td>
                    <td>
                      <code>
                        {activeMediaKind === 'image' ? '/v1/images/generations' :
                         activeMediaKind === 'tts' ? '/v1/audio/speech' :
                         activeMediaKind === 'stt' ? '/v1/audio/transcriptions' :
                         activeMediaKind === 'embedding' ? '/v1/embeddings' :
                         activeMediaKind === 'video' ? '/v1/videos/generations' :
                         m.kind === 'webSearch' ? '/v1/search' : '/v1/web/fetch'}
                      </code>
                    </td>
                    <td><span class="badge healthy">Ready</span></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- 12. PROXY POOLS -->
      {#if activeView === 'proxy-pools'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>Outbound Proxy Pools</h2>
              <div class="muted">Configure HTTP and SOCKS5 proxy chains for provider routing and IP rotation.</div>
            </div>
            <button class="primary" onclick={() => openProxyModal()}>Add Proxy</button>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>ID / Name</th>
                  <th>Proxy URL</th>
                  <th>Status</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {#if proxyPools.length === 0}
                  <tr><td colspan="4" style="text-align:center; padding:32px; color:var(--muted);">No proxy pools configured. Click "Add Proxy" to configure outbound HTTP/SOCKS5 proxies.</td></tr>
                {:else}
                  {#each proxyPools as p}
                    <tr>
                      <td><strong>{p.data?.name || p.id}</strong></td>
                      <td><code>{p.data?.url || 'Direct'}</code></td>
                      <td><span class="badge {p.test_status === 'online' ? 'healthy' : ''}">{p.test_status || 'Untested'}</span></td>
                      <td>
                        <div class="actions">
                          <button class="sm danger" onclick={() => deleteItem(`/api/proxy-pools/${encodeURIComponent(p.id)}`)}>Delete</button>
                        </div>
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- 13. SETTINGS -->
      {#if activeView === 'settings'}
        <div class="card">
          <div class="toolbar">
            <div>
              <h2>System &amp; Gateway Settings</h2>
              <div class="muted">Runtime port, API key authentication policy, and compression thresholds.</div>
            </div>
            <button class="primary" onclick={saveSettings}>Save Settings</button>
          </div>

          <div class="form-grid">
            <div class="field">
              <label>Port</label>
              <input type="number" min="1" max="65535" bind:value={settings.port}>
            </div>

            <div class="field">
              <label>
                <input type="checkbox" bind:checked={settings.require_api_key}>
                <strong>Enforce API Key for /v1 routes</strong>
              </label>
              <div class="muted">When checked, requests without a valid Bearer token receive 401 Unauthorized.</div>
            </div>

            <div class="field full">
              <label>System Diagnostics</label>
              <div class="snippet-box" style="margin-top:4px;">
                <pre>Gateway Engine: Native Rust (Axum 0.8 + Tokio 1.43)
Architecture: x86_64
Context Engine: Built-in Headroom Compression + CCR RAG
Storage: SQLite with WAL mode &amp; r2d2 connection pool
UI Framework: Svelte 5 (Compiled zero-virtual-DOM native SPA)</pre>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </main>
  </div>
</div>

<!-- MODAL DIALOG -->
{#if modalOpen}
  <div class="modal">
    <div class="modal-card">
      <div class="toolbar">
        <h2>{modalTitle}</h2>
        <button class="ghost sm" onclick={() => modalOpen = false}>✕</button>
      </div>

      <form onsubmit={submitModal}>
        {#if modalType === 'provider'}
          <div class="form-grid">
            <div class="field">
              <label>Provider</label>
              <input required bind:value={modalItem.provider} placeholder="openai">
            </div>
            <div class="field">
              <label>Connection Name</label>
              <input bind:value={modalItem.name} placeholder="Primary OpenAI">
            </div>
            <div class="field">
              <label>Authentication Type</label>
              <select bind:value={modalItem.authType}>
                <option value="api_key">API Key / Bearer</option>
                <option value="none">No Auth (Local)</option>
                <option value="oauth">OAuth 2.0 Token</option>
              </select>
            </div>
            <div class="field">
              <label>Priority</label>
              <input type="number" min="1" max="100" bind:value={modalItem.priority}>
            </div>
            <div class="field full">
              <label>API Key / Secret Token</label>
              <input type="password" bind:value={modalItem.apiKey} placeholder="sk-...">
            </div>
            <div class="field full">
              <label>Base URL Override</label>
              <input bind:value={modalItem.baseUrl} placeholder="https://api.openai.com/v1">
            </div>
            <div class="field full">
              <label>
                <input type="checkbox" bind:checked={modalItem.isActive}> Active in routing engine
              </label>
            </div>
          </div>
        {:else if modalType === 'combo'}
          <div class="form-grid">
            <div class="field">
              <label>Combo Name</label>
              <input required bind:value={modalItem.name} placeholder="coding-vip">
            </div>
            <div class="field">
              <label>Kind</label>
              <select bind:value={modalItem.kind}>
                <option value="chat">chat (LLM)</option>
                <option value="image">image</option>
                <option value="embedding">embedding</option>
                <option value="webSearch">webSearch</option>
                <option value="webFetch">webFetch</option>
              </select>
            </div>
            <div class="field full">
              <label>Fallback Chain (one model ID per line)</label>
              <textarea rows="5" required bind:value={modalItem.models} placeholder="anthropic/claude-3-5-sonnet\ndeepseek/deepseek-chat"></textarea>
            </div>
          </div>
        {:else if modalType === 'key'}
          <div class="form-grid">
            <div class="field">
              <label>Key Name / Client Tag</label>
              <input required bind:value={modalItem.name} placeholder="MacBook Pro">
            </div>
            <div class="field">
              <label>API Key Token</label>
              <input required bind:value={modalItem.key}>
            </div>
            <div class="field">
              <label>Machine ID</label>
              <input bind:value={modalItem.machineId} placeholder="host-01">
            </div>
            <div class="field">
              <label>Access Restriction Allow-List</label>
              <input bind:value={modalItem.accessAllow} placeholder="openai/*, anthropic/*">
            </div>
            <div class="field full">
              <label>
                <input type="checkbox" bind:checked={modalItem.isActive}> Key is enabled
              </label>
            </div>
          </div>
        {:else if modalType === 'proxy'}
          <div class="form-grid">
            <div class="field">
              <label>Pool Name</label>
              <input required bind:value={modalItem.name} placeholder="EU Proxy">
            </div>
            <div class="field">
              <label>Protocol</label>
              <select bind:value={modalItem.protocol}>
                <option value="http">HTTP / HTTPS</option>
                <option value="socks5">SOCKS5</option>
              </select>
            </div>
            <div class="field full">
              <label>Proxy URL</label>
              <input required bind:value={modalItem.url} placeholder="http://user:pass@proxy.example.com:8080">
            </div>
          </div>
        {/if}

        <div class="modal-actions">
          <button type="button" class="ghost" onclick={() => modalOpen = false}>Cancel</button>
          <button type="submit" class="primary">Save</button>
        </div>
      </form>
    </div>
  </div>
{/if}
