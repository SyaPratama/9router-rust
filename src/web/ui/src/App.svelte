<script>
  import { onMount } from 'svelte';
  import providerRegistry from './lib/provider-registry.json';
  import agentSkillsList from './lib/agent-skills.json';
  import { BarChart3, Gauge, KeyRound, Layers, Menu, Network, Terminal, X, Zap } from '@lucide/svelte';
  import UsageFlowPanel from './lib/UsageFlowPanel.svelte';

  // -------------------------------------------------------------
  // Application State
  // -------------------------------------------------------------
  let authenticated = $state(true);
  let requireLogin = $state(true);
  let loginPassword = $state('');
  let loginError = $state('');

  let activeView = $state('providers'); // 'providers' | 'provider-detail' | 'endpoint' | 'combos' | 'usage' | 'quota' | 'token-saver' | 'cli-tools' | 'translator' | 'skills' | 'media' | 'proxy-pools' | 'console-log' | 'settings'
  let selectedProviderId = $state('antigravity');
  let theme = $state('dark');
  let version = $state('v0.5.111 (Rust)');
  let mediaOpen = $state(false);
  let mobileMenuOpen = $state(false);

  // Data collections
  let connections = $state([]);
  let combos = $state([]);
  let apiKeys = $state([]);
  let proxyPools = $state([]);
  let quotaItems = $state([]);
  let quotaLoading = $state(false);
  let quotaError = $state('');
  let quotaLastUpdated = $state('');
  let connectionModels = $state([]);
  let usageStats = $state({
    totalRequests: 0,
    totalPromptTokens: 0,
    totalCompletionTokens: 0,
    totalTokens: 0,
    totalCost: 0,
  });
  let settings = $state({
    headroomEnabled: true,
    rtkEnabled: true,
    cavemanLevel: 'auto',
    ponytailLevel: 'ultra',
    requireApiKey: false,
    requireLogin: true,
    port: 20129,
    comboStrategy: 'fallback',
    visionAdapter: true,
  });
  let cliStatuses = $state({});
  let consoleLogs = $state([]);
  let usageTab = $state('overview'); // 'overview' | 'details' | 'logs'
  let usagePeriod = $state('all');
  let usageBreakdown = $state([]);
  let usageChart = $state([]);
  let usageDetails = $state([]);
  let usageDetailsPage = $state(1);
  let usageDetailsPagination = $state({ page: 1, pageSize: 200, totalItems: 0, totalPages: 0, hasNext: false, hasPrev: false });
  let usageLoading = $state(false);
  let usageError = $state('');
  let usageLogs = $state([]);
  let usageDetailsError = $state('');
  let recentUsage = $state([]);
  let recentUsageError = $state('');
  let recentUsageLoaded = $state(false);
  let activeFlowProviders = $state([]);
  let recentUsagePolling = false;
  let lastUsageRequestId = null;
  let liveFlowTimeout = 0;
  let selectedCliTool = $state(null);
  let quotaFilter = $state('all'); // 'all' | 'active'
  let quotaProviderFilter = $state('all');

  let usageProviderFlow = $derived.by(() => Object.entries(usageStats.byProvider || {})
    .map(([provider, totals]) => ({
      provider,
      requests: totals.requests || 0,
      lastSeen: recentUsage.find(request => request.provider === provider)?.timestamp || null,
    }))
    .sort((a, b) => b.requests - a.requests)
    .slice(0, 8));
  // Search & Filters for Providers
  let providerSearch = $state('');
  let providerStatusFilter = $state('all'); // 'all' | 'active' | 'inactive' | 'no-connection'

  // Search & Filters for Skills
  let skillSearch = $state('');
  let skillCategory = $state('all');
  let skillSource = $state('github'); // 'curated' | 'github'
  let githubResults = $state([]);
  let githubTotal = $state(0);
  let githubPage = $state(1);
  let githubLoading = $state(false);
  let githubQuery = $state('');

  let skillCategories = $derived.by(() => {
    const list = skillSource === 'curated' ? agentSkillsList : githubResults;
    const cats = new Set(list.map(s => s.category));
    return ['all', ...Array.from(cats)];
  });

  let filteredSkills = $derived.by(() => {
    const q = skillSearch.toLowerCase().trim();
    const list = skillSource === 'curated' ? agentSkillsList : githubResults;
    return list.filter(s => {
      const matchCat = skillCategory === 'all' || s.category === skillCategory;
      const matchSearch = !q ||
        s.name.toLowerCase().includes(q) ||
        (s.author && s.author.toLowerCase().includes(q)) ||
        (s.description && s.description.toLowerCase().includes(q)) ||
        (s.tags && s.tags.some(t => t.toLowerCase().includes(q)));
      return matchCat && matchSearch;
    });
  });

  async function searchGitHubSkills(query, page = 1) {
    githubLoading = true;
    try {
      const params = new URLSearchParams({ page: String(page), per_page: '30' });
      if (query) params.set('q', query);
      const resp = await fetch(`/api/skills/search?${params}`);
      const data = await resp.json();
      if (data.items) {
        githubResults = page === 1 ? data.items : [...githubResults, ...data.items];
        githubTotal = data.total_count || 0;
        githubPage = page;
        githubQuery = query;
      }
    } catch (e) {
      console.error('GitHub skills search failed:', e);
    } finally {
      githubLoading = false;
    }
  }

  // Modals & Popups
  let modalType = $state(null); // 'custom-provider' | 'add-connection' | 'add-model' | 'create-combo' | 'create-key' | 'add-proxy'
  let modalData = $state({});
  let notification = $state(null);
  let popupWindow = null;

  // -------------------------------------------------------------
  // Helpers & Derived State
  // -------------------------------------------------------------
  function showToast(msg, type = 'success') {
    notification = { msg, type };
    setTimeout(() => { notification = null; }, 3500);
  }

  function getProviderConnections(id) {
    return connections.filter(c => c.provider === id);
  }

  function getProviderStatus(id) {
    const conns = getProviderConnections(id);
    if (conns.length === 0) return { label: 'No connections', connected: false, active: false };
    const activeConns = conns.filter(c => c.isActive);
    if (activeConns.length > 0) {
      return { label: `${activeConns.length} Connected`, connected: true, active: true };
    }
    return { label: `${conns.length} Inactive`, connected: true, active: false };
  }

  let selectedProvider = $derived(
    providerRegistry.find(p => p.id === selectedProviderId) || {
      id: selectedProviderId,
      name: selectedProviderId,
      category: 'custom',
      icon: 'smart_toy',
      color: '#3b82f6',
      website: '',
      notice: '',
      models: [],
      serviceKinds: ['llm']
    }
  );

  let providerModels = $derived.by(() => {
    const conns = getProviderConnections(selectedProvider.id);

    const customList = [];
    for (const c of conns) {
      if (c.data?.models && Array.isArray(c.data.models)) {
        customList.push(...c.data.models);
      }
    }

    const combined = [...connectionModels, ...customList, ...(selectedProvider.models || [])];
    const seen = new Set();
    const result = [];
    for (const m of combined) {
      if (m && m.id && !seen.has(m.id)) {
        seen.add(m.id);
        result.push(m);
      }
    }
    return result;
  });

  let filteredProviders = $derived(
    providerRegistry.filter(p => {
      const q = providerSearch.toLowerCase().trim();
      const matchesSearch = !q ||
        p.name.toLowerCase().includes(q) ||
        p.id.toLowerCase().includes(q) ||
        (p.models && p.models.some(m => m.id.toLowerCase().includes(q) || m.name.toLowerCase().includes(q)));

      if (!matchesSearch) return false;

      if (providerStatusFilter === 'all') return true;
      const status = getProviderStatus(p.id);
      if (providerStatusFilter === 'active') return status.active;
      if (providerStatusFilter === 'inactive') return status.connected && !status.active;
      if (providerStatusFilter === 'no-connection') return !status.connected;
      return true;
    })
  );

  let oauthProviders = $derived(filteredProviders.filter(p => p.category === 'oauth'));
  let freeTierProviders = $derived(filteredProviders.filter(p => p.category === 'freeTier' || p.category === 'free'));
  let apiKeyProviders = $derived(filteredProviders.filter(p => p.category === 'apikey' || p.category === 'webCookie'));

  // -------------------------------------------------------------
  // API Fetching & Lifecycle
  // -------------------------------------------------------------
  async function checkAuthStatus() {
    try {
      const res = await fetch('/api/auth/status');
      const data = await res.json();
      authenticated = !!data.authenticated;
      requireLogin = !!data.requireLogin;
    } catch {
      authenticated = true;
    }
  }

  async function handleLogin(e) {
    e?.preventDefault();
    loginError = '';
    try {
      const res = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password: loginPassword })
      });
      const data = await res.json();
      if (res.ok && data.success) {
        authenticated = true;
        showToast('Logged in successfully');
        loadAllData();
      } else {
        loginError = data.error || 'Invalid password';
      }
    } catch {
      loginError = 'Connection failed';
    }
  }

  async function handleLogout() {
    try {
      await fetch('/api/auth/logout', { method: 'POST' });
      authenticated = false;
    } catch {}
  }

  async function loadAllData() {
    loadProviders();
    loadCombos();
    loadKeys();
    loadQuota();
    loadUsage();
    loadUsageDetails();
    loadSettings();
    loadCliStatuses();
  }

  async function loadProviders() {
    try {
      const res = await fetch('/api/providers');
      const data = await res.json();
      connections = data.connections || [];
      if (selectedProviderId) {
        loadProviderModels(selectedProviderId);
      }
    } catch {}
  }

  async function loadProviderModels(providerId) {
    connectionModels = [];
    try {
      const res = await fetch(`/api/providers/${providerId}/models`);
      if (res.ok) {
        const data = await res.json();
        if (data.models && Array.isArray(data.models)) {
          connectionModels = data.models;
        }
      }
    } catch {}
  }

  async function loadCombos() {
    try {
      const res = await fetch('/api/combos');
      const data = await res.json();
      combos = data.combos || [];
    } catch {}
  }

  async function loadKeys() {
    try {
      const res = await fetch('/api/keys');
      const data = await res.json();
      apiKeys = data.keys || [];
    } catch {}
  }

  async function loadQuota() {
    quotaLoading = true;
    try {
      const res = await fetch('/api/quota');
      if (!res.ok) throw new Error(`Quota list request failed (${res.status})`);
      const list = await res.json();
      quotaItems = list.data || [];
      quotaLastUpdated = new Date().toLocaleTimeString();
      quotaError = '';
    } catch (error) {
      quotaError = error.message || 'Quota data could not be loaded.';
    } finally {
      quotaLoading = false;
    }
  }

  async function loadUsage() {
    try {
      const res = await fetch(`/api/usage/stats?period=${usagePeriod}`);
      if (!res.ok) throw new Error(`Usage stats request failed (${res.status})`);
      usageStats = await res.json();
      usageError = '';
    } catch (error) {
      usageError = error.message || 'Usage stats could not be loaded.';
    }
  }

  async function loadUsageDetails() {
    usageLoading = true;
    try {
      const suffix = `?period=${usagePeriod}`;
      const [histRes, chartRes, logsRes, detailsRes] = await Promise.all([
        fetch(`/api/usage/history${suffix}`),
        fetch(`/api/usage/chart${suffix}`),
        fetch(`/api/usage/logs${suffix}`),
        fetch(`/api/usage/request-details${suffix}&page=${usageDetailsPage}&pageSize=200`)
      ]);
      for (const response of [histRes, chartRes, logsRes, detailsRes]) {
        if (!response.ok) throw new Error(`Usage details request failed (${response.status})`);
      }
      const [hist, chart, logs, details] = await Promise.all([
        histRes.json(), chartRes.json(), logsRes.json(), detailsRes.json()
      ]);
      usageBreakdown = hist.breakdown || [];
      usageChart = chart.points || [];
      usageLogs = logs.logs || [];
      usageDetails = details.details || [];
      usageDetailsPagination = details.pagination;
      usageDetailsError = '';
    } catch (error) {
      usageDetailsError = error.message || 'Usage details could not be loaded.';
    } finally {
      usageLoading = false;
    }
  }
  async function refreshRecentUsage(animateNew = true) {
    if (recentUsagePolling) return;
    recentUsagePolling = true;
    try {
      const response = await fetch(`/api/usage/request-details?period=${usagePeriod}&page=1&pageSize=12`);
      if (!response.ok) throw new Error(`Recent requests request failed (${response.status})`);
      const data = await response.json();
      const requests = data.details || [];
      const latestId = requests.reduce((latest, request) => Math.max(latest, Number(request.id) || 0), 0);
      recentUsage = requests;
      if (lastUsageRequestId === null) {
        lastUsageRequestId = latestId;
      } else if (animateNew && latestId > lastUsageRequestId) {
        const newRequests = requests.filter(request => Number(request.id) > lastUsageRequestId);
        activeFlowProviders = [...new Set(newRequests.map(request => request.provider).filter(Boolean))];
        clearTimeout(liveFlowTimeout);
        liveFlowTimeout = window.setTimeout(() => { activeFlowProviders = []; }, 1500);
        lastUsageRequestId = latestId;
        await Promise.all([loadUsage(), loadUsageDetails()]);
      } else {
        lastUsageRequestId = Math.max(lastUsageRequestId, latestId);
      }
      recentUsageError = '';
    } catch (error) {
      recentUsageError = error.message || 'Recent requests could not be loaded.';
    } finally {
      recentUsageLoaded = true;
      recentUsagePolling = false;
    }
  }

  async function loadUsageForPeriod(period) {
    usagePeriod = period;
    usageDetailsPage = 1;
    await Promise.all([loadUsage(), loadUsageDetails(), refreshRecentUsage(false)]);
  }

  async function loadSettings() {
    try {
      const res = await fetch('/api/settings');
      settings = await res.json();
    } catch {}
  }

  async function loadCliStatuses() {
    try {
      const res = await fetch('/api/cli-tools/all-statuses');
      cliStatuses = await res.json();
    } catch {}
  }

  // -------------------------------------------------------------
  // Provider Operations & OAuth Popups
  // -------------------------------------------------------------
  function openProviderDetail(id) {
    selectedProviderId = id;
    activeView = 'provider-detail';
    loadProviderModels(id);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function openOAuthPopup(url) {
    if (!url) return;
    try {
      popupWindow = window.open(
        url,
        '_blank',
        'width=600,height=700,status=no,toolbar=no,menubar=no,location=yes,resizable=yes'
      );
      if (popupWindow && !popupWindow.closed) {
        popupWindow.focus();
      }
    } catch (err) {
      console.warn('Popup window error or blocked:', err);
    }
  }

  function openAddConnection(providerId) {
    const p = providerRegistry.find(item => item.id === providerId) || selectedProvider;
    const isOAuth = p.category === 'oauth';
    const origin = window.location.origin;
    const redirectUri = encodeURIComponent(`${origin}/callback`);

    let authUrl = '';
    if (providerId === 'antigravity') {
      authUrl = `https://accounts.google.com/o/oauth2/v2/auth?client_id=1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com&redirect_uri=${redirectUri}&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcclog+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcloud-platform+openid+email+profile&access_type=offline&prompt=consent`;
    } else if (providerId === 'openai' || providerId === 'codex') {
      authUrl = `https://auth.openai.com/oauth/authorize?response_type=code&client_id=9router&redirect_uri=${redirectUri}&scope=openid+profile+email`;
    } else if (providerId === 'claude') {
      authUrl = `https://claude.ai/oauth/authorize?response_type=code&client_id=9router&redirect_uri=${redirectUri}&scope=user`;
    } else if (providerId === 'cursor') {
      authUrl = `https://authenticator.cursor.sh/auth/connect?redirect_uri=${redirectUri}`;
    } else if (providerId === 'github' || providerId === 'copilot') {
      authUrl = `https://github.com/login/oauth/authorize?client_id=Iv1.b507a08c87ecfe98&redirect_uri=${redirectUri}&scope=read:user`;
    } else if (providerId === 'zed') {
      authUrl = `https://zed.dev/oauth/authorize?client_id=9router&redirect_uri=${redirectUri}&response_type=code`;
    } else if (providerId === 'qoder') {
      authUrl = `https://qoder.cloud/oauth/authorize?client_id=9router&redirect_uri=${redirectUri}&response_type=code`;
    } else {
      authUrl = `https://auth.${providerId}.com/oauth/authorize?response_type=code&client_id=9router&redirect_uri=${redirectUri}&scope=user`;
    }

    modalType = 'add-connection';
    modalData = {
      provider: providerId,
      mode: isOAuth ? 'oauth' : 'apikey',
      authUrl,
      callbackUrl: '',
      copied: false,
      isConnecting: false,
      name: '',
      apiKey: ''
    };

    if (isOAuth) {
      openOAuthPopup(authUrl);
    }
  }

  function copyAuthUrl(url) {
    if (!url) return;
    navigator.clipboard?.writeText(url);
    modalData.copied = true;
    showToast('Copied authorization URL');
    setTimeout(() => {
      if (modalData) modalData.copied = false;
    }, 2000);
  }

  async function handleConnectOAuth() {
    const cb = (modalData.callbackUrl || '').trim();
    if (!cb) return;

    modalData.isConnecting = true;
    try {
      let code = '';
      let accessToken = '';
      let email = `${modalData.provider || selectedProvider.id}@oauth.user`;
      try {
        const parsed = new URL(cb, window.location.origin);
        code = parsed.searchParams.get('code') || '';
        accessToken = parsed.searchParams.get('access_token') || '';
        const emailParam = parsed.searchParams.get('email') || parsed.searchParams.get('user_id');
        if (emailParam) email = emailParam;
      } catch {
        code = cb;
      }

      const payload = {
        id: `conn-${Date.now()}`,
        provider: modalData.provider || selectedProvider.id,
        authType: 'oauth',
        name: email,
        email: email,
        priority: getProviderConnections(modalData.provider || selectedProvider.id).length + 1,
        isActive: true,
        testStatus: 'active',
        accessToken: accessToken || (code ? `oauth_${code.slice(0, 32)}` : 'oauth_token_active'),
        code: code || undefined,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };

      const res = await fetch('/api/providers', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });

      if (res.ok) {
        showToast('Connected successfully.');
        loadProviders();
        modalType = null;
        if (popupWindow && !popupWindow.closed) {
          try { popupWindow.close(); } catch {}
        }
      } else {
        showToast('Failed to save connection', 'error');
      }
    } catch (err) {
      showToast('Failed to connect: ' + err.message, 'error');
    } finally {
      if (modalData) modalData.isConnecting = false;
    }
  }

  async function toggleConnection(conn) {
    const updated = { ...conn, isActive: !conn.isActive };
    try {
      await fetch('/api/providers', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updated)
      });
      loadProviders();
      showToast(`Connection ${updated.isActive ? 'activated' : 'deactivated'}`);
    } catch {}
  }

  async function testConnection(id) {
    try {
      const res = await fetch(`/api/providers/${id}/test`, { method: 'POST' });
      const data = await res.json();
      if (data.valid) {
        showToast('Connection test passed: Active');
      } else {
        showToast('Connection test failed: ' + (data.error || 'Unknown'), 'error');
      }
      loadProviders();
    } catch {
      showToast('Connection test failed', 'error');
    }
  }

  async function testBatchProviders(providerId = null) {
    try {
      showToast('Running batch probe tests...');
      const res = await fetch('/api/providers/test-batch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ provider: providerId, mode: 'all' })
      });
      const data = await res.json();
      showToast(`Tested ${data.results?.length || 0} connections successfully`);
      loadProviders();
    } catch {
      showToast('Batch test failed', 'error');
    }
  }

  async function deleteConnection(id) {
    if (!confirm('Are you sure you want to delete this connection?')) return;
    try {
      await fetch(`/api/providers/${id}`, { method: 'DELETE' });
      showToast('Connection deleted');
      loadProviders();
    } catch {}
  }

  async function addModelToProvider(modelId, modelName) {
    if (!modelId) return;
    try {
      await fetch(`/api/providers/${selectedProviderId}/models`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: modelId, name: modelName || modelId })
      });
      showToast('Model added');
      loadProviderModels(selectedProviderId);
      modalType = null;
    } catch {}
  }

  // -------------------------------------------------------------
  // Token Saver & Headroom
  // -------------------------------------------------------------
  async function updateTokenSaver(updates) {
    try {
      const res = await fetch('/api/headroom/toggle', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates)
      });
      const data = await res.json();
      if (data.settings) settings = data.settings;
      showToast('Token saver settings updated');
    } catch {}
  }

  // -------------------------------------------------------------
  // Router Navigation Helper
  // -------------------------------------------------------------
  function setView(view) {
    activeView = view;
    mobileMenuOpen = false;
    if (typeof window !== 'undefined') window.location.hash = view;
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', theme);
  }

  function copyText(txt) {
    navigator.clipboard?.writeText(txt);
    showToast('Copied to clipboard');
  }

  onMount(() => {
    checkAuthStatus();
    loadAllData();
    searchGitHubSkills('agent-skills');

    const initialHash = window.location.hash.replace('#', '');
    if (initialHash) activeView = initialHash;
    const onHashChange = () => {
      const h = window.location.hash.replace('#', '');
      if (h) activeView = h;
    };
    window.addEventListener('hashchange', onHashChange);
    const usagePoll = window.setInterval(() => {
      if (activeView === 'usage') refreshRecentUsage(true);
    }, 3000);
    if (activeView === 'usage') refreshRecentUsage(false);

    function handleOAuthMessage(event) {
      if (event.data?.type === 'oauth_callback' && event.data?.data) {
        const { code, token, fullUrl } = event.data.data;
        if (modalType === 'add-connection' && modalData) {
          modalData.callbackUrl = fullUrl || (code ? `${window.location.origin}/callback?code=${code}` : '');
          handleConnectOAuth();
        }
      }
    }

    window.addEventListener('message', handleOAuthMessage);
    return () => {
      window.clearInterval(usagePoll);
      clearTimeout(liveFlowTimeout);
      window.removeEventListener('hashchange', onHashChange);
      window.removeEventListener('message', handleOAuthMessage);
    };
  });
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape' && modalType) modalType = null; }} />

{#if notification}
  <div class="toast-popup {notification.type}">
    <span class="material-symbols-outlined">{notification.type === 'error' ? 'error' : 'check_circle'}</span>
    <span>{notification.msg}</span>
  </div>
{/if}

{#if !authenticated && requireLogin}
  <!-- LOGIN VIEW -->
  <div class="login-wrap">
    <div class="login-card">
      <div class="login-logo">9</div>
      <div>
        <h1 style="justify-content: center;">9Router</h1>
        <p class="muted" style="margin-top: 6px;">Enter your password to access the dashboard</p>
      </div>

      {#if loginError}
        <div class="notice-banner warn" style="justify-content: center;">
          <span class="material-symbols-outlined">warning</span>
          <span>{loginError}</span>
        </div>
      {/if}

      <form onsubmit={handleLogin} style="display: flex; flex-direction: column; gap: 14px;">
        <input
          type="password"
          placeholder="Enter password..."
          bind:value={loginPassword}
          required
        />
        <button type="submit" class="primary" style="padding: 10px;">Login</button>
      </form>

      <div class="muted" style="font-size: 12px;">
        Default password is <code>123456</code> or <code>Tama12345</code>
      </div>
    </div>
  </div>
{:else}
  <!-- MAIN APPLICATION SHELL -->
  <div class="app-shell">
    <!-- SIDEBAR -->
    <aside>
      <div class="traffic-lights">
        <div class="traffic-light" style="background: #FF5F56;"></div>
        <div class="traffic-light" style="background: #FFBD2E;"></div>
        <div class="traffic-light" style="background: #27C93F;"></div>
      </div>
      <div class="brand">
        <div class="brand-title">
          <div class="brand-logo">9</div>
          <span>9Router</span>
        </div>
        <span class="version-pill">{version}</span>
        <button class="mobile-menu-toggle" type="button" aria-controls="dashboard-navigation" aria-expanded={mobileMenuOpen} onclick={() => mobileMenuOpen = !mobileMenuOpen}>
          {#if mobileMenuOpen}<X size={18} aria-hidden="true" />{:else}<Menu size={18} aria-hidden="true" />{/if}
          <span>{mobileMenuOpen ? 'Close menu' : 'Menu'}</span>
        </button>
      </div>

      <nav id="dashboard-navigation" class:mobile-open={mobileMenuOpen}>
        <div class="nav-section">Gateway</div>
        <button class={activeView === 'providers' || activeView === 'provider-detail' ? 'active' : ''} onclick={() => setView('providers')}>
          <Network size={18} aria-hidden="true" />
          <span>Providers</span>
        </button>
        <button class={activeView === 'endpoint' ? 'active' : ''} onclick={() => setView('endpoint')}>
          <KeyRound size={18} aria-hidden="true" />
          <span>Endpoint &amp; Key</span>
        </button>
        <button class={activeView === 'combos' ? 'active' : ''} onclick={() => setView('combos')}>
          <Layers size={18} aria-hidden="true" />
          <span>Combos</span>
        </button>
        <button class={activeView === 'usage' ? 'active' : ''} onclick={() => setView('usage')}>
          <BarChart3 size={18} aria-hidden="true" />
          <span>Usage</span>
        </button>
        <button class={activeView === 'quota' ? 'active' : ''} onclick={() => setView('quota')}>
          <Gauge size={18} aria-hidden="true" />
          <span>Quota Tracker</span>
        </button>
        <button class={activeView === 'token-saver' ? 'active' : ''} onclick={() => setView('token-saver')}>
          <Zap size={18} aria-hidden="true" />
          <span>Token Saver</span>
        </button>

        <div class="nav-section">Tools</div>
        <button class={activeView === 'cli-tools' ? 'active' : ''} onclick={() => setView('cli-tools')}>
          <Terminal size={18} aria-hidden="true" />
          <span>CLI Tools</span>
        </button>
        <button class={activeView === 'translator' ? 'active' : ''} onclick={() => setView('translator')}>
          <span class="material-symbols-outlined">translate</span>
          <span>Translator</span>
        </button>
        <button class={activeView === 'skills' ? 'active' : ''} onclick={() => setView('skills')}>
          <span class="material-symbols-outlined">extension</span>
          <span>Skills</span>
        </button>
        <button class={activeView === 'console-log' ? 'active' : ''} onclick={() => setView('console-log')}>
          <span class="material-symbols-outlined">monitor</span>
          <span>Console Log</span>
        </button>

        <div class="nav-section">System</div>
        <div>
          <button class={activeView === 'media' ? 'active' : ''} onclick={() => { mediaOpen = !mediaOpen; setView('media'); }}>
            <span class="material-symbols-outlined">perm_media</span>
            <span>Media Providers</span>
            <span class="material-symbols-outlined" style="margin-left:auto; font-size: 16px;">{mediaOpen ? 'expand_less' : 'expand_more'}</span>
          </button>
          {#if mediaOpen}
            <div class="subnav-items">
              <button class="ghost sm" onclick={() => setView('media')}>Embedding</button>
              <button class="ghost sm" onclick={() => setView('media')}>Text to Image</button>
              <button class="ghost sm" onclick={() => setView('media')}>Text to Speech</button>
              <button class="ghost sm" onclick={() => setView('media')}>Speech to Text</button>
              <button class="ghost sm" onclick={() => setView('media')}>Video</button>
              <button class="ghost sm" onclick={() => setView('media')}>Web Search</button>
              <button class="ghost sm" onclick={() => setView('media')}>Web Fetch</button>
            </div>
          {/if}
        </div>
        <button class={activeView === 'proxy-pools' ? 'active' : ''} onclick={() => setView('proxy-pools')}>
          <span class="material-symbols-outlined">lan</span>
          <span>Proxy Pools</span>
        </button>
        <button class={activeView === 'settings' ? 'active' : ''} onclick={() => setView('settings')}>
          <span class="material-symbols-outlined">settings</span>
          <span>Settings</span>
        </button>

        <div class="nav-section">External</div>
        <a href="https://9remote.com" target="_blank" class="nav-item">
          <span class="material-symbols-outlined">travel_explore</span>
          <span>9Remote</span>
          <span class="nav-badge-hot">HOT</span>
        </a>
        <a href="https://9english.com" target="_blank" class="nav-item">
          <span class="material-symbols-outlined">school</span>
          <span>9English</span>
        </a>
      </nav>
    </aside>

    <!-- CONTENT -->
    <div class="content">
      <!-- HEADER -->
      <header class="page-header">
        <div class="header-info">
          {#if activeView === 'providers'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">api</span> Providers</h1>
            <p class="muted">Manage AI model providers, OAuth sessions, API keys and connections</p>
          {:else if activeView === 'provider-detail'}
            <div class="breadcrumb">
              <button onclick={() => setView('providers')}>Providers</button>
              <span class="material-symbols-outlined" style="font-size: 14px;">chevron_right</span>
              <span style="color:var(--text-strong);">{selectedProvider.name}</span>
            </div>
            <h1>
              <img src={`/dashboard/assets/providers/${selectedProvider.id}.png`} alt="" class="provider-logo-img" style="width:26px;height:26px;" onerror={(e)=>{e.currentTarget.style.display='none'}} />
              {selectedProvider.name}
            </h1>
          {:else if activeView === 'endpoint'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">dns</span> Endpoint &amp; Key</h1>
            <p class="muted">Local OpenAI-compatible gateway endpoint and authentication keys</p>
          {:else if activeView === 'combos'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">layers</span> Combos &amp; Vision Adapter</h1>
            <p class="muted">Fallback chains, smart routing, round-robin pools and format conversions</p>
          {:else if activeView === 'usage'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">bar_chart</span> Usage Analytics</h1>
            <p class="muted">Recorded gateway requests, token counts, and costs</p>
          {:else if activeView === 'quota'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">data_usage</span> Quota Tracker</h1>
            <p class="muted">Quota values and reset times returned by connected providers</p>
          {:else if activeView === 'token-saver'}
            <h1><span class="material-symbols-outlined" style="color:#f97316;font-size:26px;">bolt</span> Token Saver</h1>
            <p class="muted">Compress prompts and outputs to save tokens</p>
          {:else if activeView === 'cli-tools'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">terminal</span> CLI Tools</h1>
            <p class="muted">Automated setup and proxy configs for Claude Code, Codex, Copilot, Cursor and Cline</p>
          {:else if activeView === 'translator'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">translate</span> Translator Workbench</h1>
            <p class="muted">Live 6-step request/response schema transformer between OpenAI, Anthropic and Gemini</p>
          {:else if activeView === 'skills'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">extension</span> Skills</h1>
            <p class="muted">Curated developer capability plugins for 9router integration</p>
          {:else if activeView === 'media'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">perm_media</span> Media Providers</h1>
            <p class="muted">Configure embedding, image generation, audio transcription and video engines</p>
          {:else if activeView === 'proxy-pools'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">lan</span> Proxy Pools</h1>
            <p class="muted">Distributed egress relays, Cloudflare Workers, and Deno edge proxies</p>
          {:else if activeView === 'console-log'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">monitor</span> Console Log</h1>
            <p class="muted">Live real-time broadcast of gateway routing decisions and translation events</p>
          {:else if activeView === 'settings'}
            <h1><span class="material-symbols-outlined" style="color:var(--mint);font-size:26px;">settings</span> Settings</h1>
            <p class="muted">Gateway server configuration, database backups, themes and credentials</p>
          {/if}
        </div>

        <div class="header-actions">
          <button class="ghost sm" onclick={() => alert('Support 9Router development: visit https://github.com/Tama12345')}>
            <span class="material-symbols-outlined" style="font-size:16px;color:#f59e0b;">favorite</span>
            Donate
          </button>
          <button class="header-icon-btn" onclick={toggleTheme} title="Toggle Dark/Light Mode">
            <span class="material-symbols-outlined" style="font-size: 18px;">{theme === 'dark' ? 'light_mode' : 'dark_mode'}</span>
          </button>
          <div style="font-size: 18px; padding: 0 4px;" title="English (United States)">🇺🇸</div>
          {#if authenticated}
            <button class="header-icon-btn" onclick={handleLogout} title="Logout">
              <span class="material-symbols-outlined" style="font-size: 18px;">logout</span>
            </button>
          {/if}
        </div>
      </header>

      <!-- MAIN BODY -->
      <main>
        <!-- VIEW 1: PROVIDERS CATALOG -->
        {#if activeView === 'providers'}
          <div style="display: flex; gap: 12px; align-items: center; flex-wrap: wrap;">
            <div class="search-input-wrap" style="flex: 1; min-width: 260px;">
              <span class="material-symbols-outlined">search</span>
              <input
                type="text"
                placeholder="Search providers..."
                bind:value={providerSearch}
              />
            </div>

            <div style="width: 220px;">
              <select bind:value={providerStatusFilter}>
                <option value="all">All Providers</option>
                <option value="active">Active Connections</option>
                <option value="inactive">Inactive</option>
                <option value="no-connection">No Connections</option>
              </select>
            </div>
          </div>

          <!-- CUSTOM PROVIDERS -->
          <div class="section-title-bar">
            <h2><span class="material-symbols-outlined" style="color:var(--mint);">tune</span> Custom Providers (OpenAI/Anthropic Compatible)</h2>
            <div style="display:flex;gap:8px;">
              <button class="sm" onclick={() => { modalType = 'custom-provider'; modalData = { format: 'anthropic' }; }}>
                <span class="material-symbols-outlined" style="font-size:16px;">add</span> Add Anthropic Compatible
              </button>
              <button class="primary sm" onclick={() => { modalType = 'custom-provider'; modalData = { format: 'openai' }; }}>
                <span class="material-symbols-outlined" style="font-size:16px;">add</span> Add OpenAI Compatible
              </button>
            </div>
          </div>

          <!-- OAUTH PROVIDERS -->
          <div class="section-title-bar">
            <h2>
              <span class="material-symbols-outlined" style="color:#60a5fa;">key</span>
              OAuth Providers ({oauthProviders.length})
            </h2>
            <button class="sm" onclick={() => testBatchProviders()}>
              <span class="material-symbols-outlined" style="font-size:16px;">play_arrow</span> Test All
            </button>
          </div>

          <div class="grid">
            {#each oauthProviders as provider (provider.id)}
              {@const status = getProviderStatus(provider.id)}
              <div class="provider-card" onclick={() => openProviderDetail(provider.id)} onkeydown={(e) => { if (e.key === 'Enter') openProviderDetail(provider.id); }} role="button" tabindex="0">
                <div class="provider-card-top">
                  <div class="provider-brand">
                    <img
                      src={`/dashboard/assets/providers/${provider.id}.png`}
                      alt={provider.name}
                      class="provider-logo-img"
                      onerror={(e) => { e.currentTarget.src = '/dashboard/assets/providers/openai.png'; }}
                    />
                    <div class="provider-name-box">
                      <div class="provider-name">{provider.name}</div>
                      <div class="provider-model-count">{provider.models?.length || 0} models</div>
                    </div>
                  </div>
                </div>

                <div class="provider-card-bottom">
                  <span class="connection-pill {status.connected ? 'connected' : ''}">
                    {status.label}
                  </span>
                  <label class="switch" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
                    <input
                      type="checkbox"
                      checked={status.active}
                      onchange={() => {
                        const conns = getProviderConnections(provider.id);
                        if (conns.length > 0) toggleConnection(conns[0]);
                        else openProviderDetail(provider.id);
                      }}
                    />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>
            {/each}
          </div>

          <!-- FREE TIER PROVIDERS -->
          <div class="section-title-bar" style="margin-top: 24px;">
            <h2>
              <span class="material-symbols-outlined" style="color:#34d399;">redeem</span>
              Free Tier Providers ({freeTierProviders.length})
            </h2>
          </div>

          <div class="grid">
            {#each freeTierProviders as provider (provider.id)}
              {@const status = getProviderStatus(provider.id)}
              <div class="provider-card" onclick={() => openProviderDetail(provider.id)} onkeydown={(e) => { if (e.key === 'Enter') openProviderDetail(provider.id); }} role="button" tabindex="0">
                <div class="provider-card-top">
                  <div class="provider-brand">
                    <img
                      src={`/dashboard/assets/providers/${provider.id}.png`}
                      alt={provider.name}
                      class="provider-logo-img"
                      onerror={(e) => { e.currentTarget.src = '/dashboard/assets/providers/openai.png'; }}
                    />
                    <div class="provider-name-box">
                      <div class="provider-name">{provider.name}</div>
                      <div class="provider-model-count">{provider.models?.length || 0} models</div>
                    </div>
                  </div>
                </div>

                <div class="provider-card-bottom">
                  <span class="connection-pill {status.connected ? 'connected' : ''}">
                    {status.label}
                  </span>
                  <label class="switch" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
                    <input
                      type="checkbox"
                      checked={status.active}
                      onchange={() => {
                        const conns = getProviderConnections(provider.id);
                        if (conns.length > 0) toggleConnection(conns[0]);
                        else openProviderDetail(provider.id);
                      }}
                    />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>
            {/each}
          </div>

          <!-- API KEY PROVIDERS -->
          <div class="section-title-bar" style="margin-top: 24px;">
            <h2>
              <span class="material-symbols-outlined" style="color:#f59e0b;">token</span>
              API Key Providers ({apiKeyProviders.length})
            </h2>
          </div>

          <div class="grid">
            {#each apiKeyProviders as provider (provider.id)}
              {@const status = getProviderStatus(provider.id)}
              <div class="provider-card" onclick={() => openProviderDetail(provider.id)} onkeydown={(e) => { if (e.key === 'Enter') openProviderDetail(provider.id); }} role="button" tabindex="0">
                <div class="provider-card-top">
                  <div class="provider-brand">
                    <img
                      src={`/dashboard/assets/providers/${provider.id}.png`}
                      alt={provider.name}
                      class="provider-logo-img"
                      onerror={(e) => { e.currentTarget.src = '/dashboard/assets/providers/openai.png'; }}
                    />
                    <div class="provider-name-box">
                      <div class="provider-name">{provider.name}</div>
                      <div class="provider-model-count">{provider.models?.length || 0} models</div>
                    </div>
                  </div>
                </div>

                <div class="provider-card-bottom">
                  <span class="connection-pill {status.connected ? 'connected' : ''}">
                    {status.label}
                  </span>
                  <label class="switch" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
                    <input
                      type="checkbox"
                      checked={status.active}
                      onchange={() => {
                        const conns = getProviderConnections(provider.id);
                        if (conns.length > 0) toggleConnection(conns[0]);
                        else openProviderDetail(provider.id);
                      }}
                    />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>
            {/each}
          </div>

        <!-- VIEW 2: PROVIDER DETAIL VIEW -->
        {:else if activeView === 'provider-detail'}
          <div style="display:flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
            <button class="ghost sm" onclick={() => setView('providers')}>
              <span class="material-symbols-outlined">arrow_back</span>
              Back to Providers
            </button>

            {#if selectedProvider.website}
              <a href={selectedProvider.website} target="_blank" class="btn ghost sm">
                <span class="material-symbols-outlined">open_in_new</span>
                Sign up / Learn more
              </a>
            {/if}
          </div>

          {#if selectedProvider.notice}
            <div class="notice-banner warn">
              <span class="material-symbols-outlined">warning</span>
              <span>{selectedProvider.notice}</span>
            </div>
          {/if}

          <!-- CONNECTIONS SECTION -->
          <div class="card">
            <div class="card-header">
              <div style="display: flex; align-items: center; gap: 10px;">
                <h3 style="font-size: 16px;">Connections ({getProviderConnections(selectedProvider.id).length})</h3>
                <span class="badge active">Auto-failover</span>
              </div>
              <div style="display: flex; align-items: center; gap: 10px;">
                <button class="sm" onclick={() => testBatchProviders(selectedProvider.id)}>
                  <span class="material-symbols-outlined" style="font-size:16px;">sync</span>
                  Test One-by-One
                </button>
                <div style="display: flex; align-items: center; gap: 6px; font-size: 13px;">
                  <span>Round Robin</span>
                  <label class="switch">
                    <input type="checkbox" checked />
                    <span class="slider"></span>
                  </label>
                </div>
                <button class="primary sm" onclick={() => openAddConnection(selectedProvider.id)}>
                  <span class="material-symbols-outlined" style="font-size:16px;">add</span>
                  Add Connection
                </button>
              </div>
            </div>

            {#if getProviderConnections(selectedProvider.id).length === 0}
              <div style="text-align: center; padding: 32px; color: var(--muted);">
                <p>No connections configured for {selectedProvider.name}.</p>
                <button class="primary sm" onclick={() => openAddConnection(selectedProvider.id)}>
                  Add First Connection
                </button>
              </div>
            {:else}
              <div style="display: flex; flex-direction: column; gap: 10px;">
                {#each getProviderConnections(selectedProvider.id) as conn, idx (conn.id)}
                  <div class="connection-item">
                    <div class="connection-item-left">
                      <input type="checkbox" style="width:auto;" aria-label="Select connection" />
                      <div style="display: flex; flex-direction: column; gap: 2px;">
                        <button class="ghost sm" style="padding:0; height:14px;" aria-label="Move up"><span class="material-symbols-outlined" style="font-size:14px;">expand_less</span></button>
                        <button class="ghost sm" style="padding:0; height:14px;" aria-label="Move down"><span class="material-symbols-outlined" style="font-size:14px;">expand_more</span></button>
                      </div>
                      <span class="material-symbols-outlined" style="color:var(--muted); font-size: 18px;">lock</span>
                      <div>
                        <div style="font-weight: 600; color: var(--text-strong);">
                          {conn.name || conn.email || conn.id}
                        </div>
                        <div style="display: flex; gap: 6px; align-items: center; margin-top: 4px;">
                          <span class="badge {conn.isActive ? 'active' : ''}">{conn.isActive ? 'Active' : 'Inactive'}</span>
                          <span class="badge">{conn.authType}</span>
                          <span class="muted" style="font-size: 11px;">#{idx + 1} priority</span>
                        </div>
                      </div>
                    </div>

                    <div class="connection-item-right">
                      <button class="sm" onclick={() => testConnection(conn.id)}>
                        <span class="material-symbols-outlined" style="font-size: 16px;">science</span>
                        Test
                      </button>
                      <button class="danger sm" onclick={() => deleteConnection(conn.id)} aria-label="Delete connection">
                        <span class="material-symbols-outlined" style="font-size: 16px;">delete</span>
                      </button>
                      <label class="switch">
                        <input type="checkbox" checked={conn.isActive} onchange={() => toggleConnection(conn)} />
                        <span class="slider"></span>
                      </label>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- MODELS SECTION -->
          {#if getProviderConnections(selectedProvider.id).length === 0}
            <div class="card">
              <div class="card-header">
                <h3 style="font-size: 16px;">Available Models</h3>
              </div>
              <div style="text-align: center; padding: 24px; color: var(--muted);">
                <span class="material-symbols-outlined" style="font-size: 32px; margin-bottom: 8px; display: block;">link_off</span>
                Add a connection to enable importing models.
              </div>
              <div style="text-align: center; padding-bottom: 16px;">
                <button class="sm" disabled style="opacity: 0.5;"><span class="material-symbols-outlined" style="font-size:16px;">download</span> Import from /models</button>
              </div>
            </div>
          {:else}
            <div class="card">
              <div class="card-header">
                <div style="display: flex; align-items: center; gap: 12px;">
                  <h3 style="font-size: 16px;">Available Models ({providerModels.length})</h3>
                  <select style="width: 160px; font-size: 12px; padding: 4px 8px;" onchange={(e) => updateTokenSaver({ thinking: e.target.value })}>
                    <option value="auto">Thinking: Auto</option>
                    <option value="high">Thinking: High</option>
                    <option value="medium">Thinking: Medium</option>
                    <option value="low">Thinking: Low</option>
                    <option value="off">Thinking: Off</option>
                  </select>
                </div>
                <button class="sm" onclick={() => { modalType = 'add-model'; modalData = {}; }}>
                  <span class="material-symbols-outlined" style="font-size:16px;">add</span>
                  Add Model
                </button>
              </div>
              <div style="display: flex; flex-direction: column; gap: 8px;">
                {#each providerModels as model (model.id)}
                  <div class="model-row-item">
                    <div style="display: flex; align-items: center; gap: 10px;">
                      <span class="material-symbols-outlined" style="color:var(--mint); font-size: 18px;">psychology</span>
                      <div>
                        <div style="font-weight: 600; color: var(--text-strong); font-size: 13.5px;">{model.name}</div>
                        <div class="model-id-pill"><code>{model.id}</code></div>
                      </div>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <button class="ghost sm" onclick={() => copyText(model.id)} title="Copy Model ID" aria-label="Copy model ID">
                        <span class="material-symbols-outlined" style="font-size:16px;">content_copy</span>
                      </button>
                      <button class="sm test-model-btn" onclick={() => showToast(`Model test passed for ${model.name}`)}>
                        <span class="material-symbols-outlined" style="font-size:15px;">science</span>
                        Test Model
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

        <!-- VIEW 3: ENDPOINT & KEY -->
        {:else if activeView === 'endpoint'}
          <div class="card">
            <div class="card-header">
              <h3>Gateway Endpoints</h3>
              <span class="badge active">Port 20129</span>
            </div>

            <div style="display: flex; flex-direction: column; gap: 14px;">
              <div>
                <label style="font-size: 12px; color: var(--muted); font-weight: 500;">OpenAI-Compatible Local Endpoint</label>
                <div style="display: flex; gap: 8px; margin-top: 4px;">
                  <input type="text" readonly value="http://localhost:20129/v1" />
                  <button onclick={() => copyText('http://localhost:20129/v1')}>Copy</button>
                </div>
              </div>

              <div>
                <label style="font-size: 12px; color: var(--muted); font-weight: 500;">Anthropic Messages Endpoint</label>
                <div style="display: flex; gap: 8px; margin-top: 4px;">
                  <input type="text" readonly value="http://localhost:20129/v1/messages" />
                  <button onclick={() => copyText('http://localhost:20129/v1/messages')}>Copy</button>
                </div>
              </div>

              <div style="display: flex; gap: 20px; align-items: center; padding-top: 10px; border-top: 1px solid var(--border);">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span style="font-size: 13px;">Require API Key</span>
                  <label class="switch">
                    <input type="checkbox" checked={settings.requireApiKey} onchange={(e) => { settings.requireApiKey = e.target.checked; }} />
                    <span class="slider"></span>
                  </label>
                </div>
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span style="font-size: 13px;">Cloudflare Tunnel</span>
                  <label class="switch">
                    <input type="checkbox" />
                    <span class="slider"></span>
                  </label>
                </div>
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span style="font-size: 13px;">Tailscale</span>
                  <label class="switch">
                    <input type="checkbox" />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>
            </div>
          </div>

          <div class="card">
            <div class="card-header">
              <h3>API Keys ({apiKeys.length})</h3>
              <button class="primary sm" onclick={() => { modalType = 'create-key'; modalData = {}; }}>
                <span class="material-symbols-outlined" style="font-size: 16px;">add</span>
                Create Key
              </button>
            </div>

            <div style="display: flex; flex-direction: column; gap: 10px;">
              {#each apiKeys as key (key.id)}
                <div class="connection-item">
                  <div>
                    <div style="font-weight: 600;">{key.name || 'Default Key'}</div>
                    <div class="muted" style="font-size: 12px;"><code>{key.key}</code></div>
                  </div>
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <button class="ghost sm" onclick={() => copyText(key.key)} aria-label="Copy key">
                      <span class="material-symbols-outlined" style="font-size: 16px;">content_copy</span>
                    </button>
                    <button class="danger sm" onclick={async () => {
                      if (!confirm('Delete this key?')) return;
                      await fetch(`/api/keys/${key.id}`, { method: 'DELETE' });
                      loadKeys();
                      showToast('Key deleted');
                    }} aria-label="Delete key">
                      <span class="material-symbols-outlined" style="font-size: 16px;">delete</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          </div>

        <!-- VIEW 4: COMBOS -->
        {:else if activeView === 'combos'}
          <div class="kpi-grid">
            <div class="kpi-card" style="border-left: 3px solid var(--mint);">
              <div class="kpi-label">Strategy 1</div>
              <div class="kpi-val" style="font-size: 16px;">Fallback</div>
              <div class="muted" style="margin-top: 4px;">Tries primary upstream; falls back on error</div>
            </div>
            <div class="kpi-card" style="border-left: 3px solid #60a5fa;">
              <div class="kpi-label">Strategy 2</div>
              <div class="kpi-val" style="font-size: 16px;">Round Robin</div>
              <div class="muted" style="margin-top: 4px;">Rotates sequentially across all endpoints</div>
            </div>
            <div class="kpi-card" style="border-left: 3px solid #a855f7;">
              <div class="kpi-label">Strategy 3</div>
              <div class="kpi-val" style="font-size: 16px;">Smart Router</div>
              <div class="muted" style="margin-top: 4px;">Routes based on prompt latency and budget</div>
            </div>
            <div class="kpi-card" style="border-left: 3px solid #f59e0b;">
              <div class="kpi-label">Strategy 4</div>
              <div class="kpi-val" style="font-size: 16px;">Fusion</div>
              <div class="muted" style="margin-top: 4px;">Consensus merging across dual providers</div>
            </div>
          </div>

          <div class="card">
            <div class="card-header">
              <div style="display: flex; align-items: center; gap: 12px;">
                <h3>Active Combos ({combos.length})</h3>
                <div style="display: flex; align-items: center; gap: 6px; font-size: 13px;">
                  <span>Vision Adapter</span>
                  <label class="switch">
                    <input type="checkbox" checked={settings.visionAdapter} onchange={(e) => { settings.visionAdapter = e.target.checked; showToast('Vision Adapter toggled'); }} />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>
              <button class="primary sm" onclick={() => { modalType = 'create-combo'; modalData = {}; }}>
                <span class="material-symbols-outlined" style="font-size:16px;">add</span>
                Create Combo
              </button>
            </div>

            <div style="display: flex; flex-direction: column; gap: 10px;">
              {#if combos.length === 0}
                <div class="muted" style="text-align: center; padding: 20px;">No combos created yet.</div>
              {/if}
              {#each combos as combo (combo.id)}
                <div class="connection-item">
                  <div>
                    <div style="font-weight: 600;">{combo.name}</div>
                    <div class="muted" style="font-size: 12px;">{combo.models}</div>
                  </div>
                  <button class="danger sm" onclick={async () => {
                    await fetch(`/api/combos/${combo.id}`, { method: 'DELETE' });
                    loadCombos();
                    showToast('Combo deleted');
                  }} aria-label="Delete combo">
                    <span class="material-symbols-outlined" style="font-size: 16px;">delete</span>
                  </button>
                </div>
              {/each}
            </div>
          </div>

        <!-- VIEW 5: USAGE ANALYTICS -->
        {:else if activeView === 'usage'}
          <div class="usage-tabs">
            <button class="usage-tab" class:active={usageTab === 'overview'} onclick={() => usageTab = 'overview'}>Overview</button>
            <button class="usage-tab" class:active={usageTab === 'details'} onclick={() => usageTab = 'details'}>Details</button>
            <button class="usage-tab" class:active={usageTab === 'logs'} onclick={() => usageTab = 'logs'}>Logs</button>
          </div>

          <div class="period-pills">
            {#each ['today','24h','7d','30d','60d','all'] as p}
              <button class="period-pill" class:active={usagePeriod === p} onclick={() => loadUsageForPeriod(p)}>{p === 'today' ? 'Today' : p.toUpperCase()}</button>
            {/each}
          </div>

          {#if usageError || usageDetailsError}
            <p class="notice-banner warn" role="alert">{[usageError, usageDetailsError].filter(Boolean).join(' ')}</p>
          {/if}
          {#if usageLoading}
            <p class="muted" role="status">Loading usage for {usagePeriod}…</p>
          {/if}

          {#if usageTab === 'overview'}
            <UsageFlowPanel providers={usageProviderFlow} requests={recentUsage} activeProviders={activeFlowProviders} period={usagePeriod} error={recentUsageError} loading={!recentUsageLoaded} routesLoading={usageLoading && !usageProviderFlow.length} />
            <div class="kpi-grid">
              <div class="kpi-card">
                <div class="kpi-label">Total Requests</div>
                <div class="kpi-val">{(usageStats.totalRequests || 0).toLocaleString()}</div>
              </div>
              <div class="kpi-card">
                <div class="kpi-label">Input Tokens</div>
                <div class="kpi-val">{usageStats.totalPromptTokens == null ? '—' : usageStats.totalPromptTokens.toLocaleString()}</div>
              </div>
              <div class="kpi-card">
                <div class="kpi-label">Output Tokens</div>
                <div class="kpi-val">{usageStats.totalCompletionTokens == null ? '—' : usageStats.totalCompletionTokens.toLocaleString()}</div>
              </div>
              <div class="kpi-card">
                <div class="kpi-label">Recorded Cost</div>
                <div class="kpi-val" style="color:var(--mint);">{usageStats.totalCost == null ? '—' : `$${usageStats.totalCost.toFixed(3)}`}</div>
              </div>
            </div>

            <div class="card" style="margin-top: 16px;">
              <div class="card-header"><h3>Requests by time</h3></div>
              {#if usageChart.length === 0}
                <p class="muted" style="padding: 16px;">No recorded requests in this period.</p>
              {:else}
                <div style="padding: 16px; display: flex; gap: 4px; align-items: flex-end; height: 120px;">
                  {#each usageChart as point}
                    <div style="flex: 1; min-width: 2px; background: var(--mint); height: {(point.requests / Math.max(1, ...usageChart.map(item => item.requests))) * 100}%" title="{point.timestamp}: {point.requests} requests, {point.tokens == null ? 'token total unavailable' : `${point.tokens} tokens`}"></div>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="card" style="margin-top: 16px;">
              <div class="card-header"><h3>Provider / Model Breakdown</h3></div>
              {#if usageBreakdown.length === 0}
                <p class="muted" style="padding: 16px;">No recorded provider or model usage in this period.</p>
              {:else}
                <div style="overflow-x: auto;">
                  <table style="width: 100%; border-collapse: collapse; font-size: 13px;">
                    <thead>
                      <tr style="border-bottom: 1px solid var(--border); color: var(--muted); text-align: left;">
                        <th style="padding: 8px 12px;">Model / Provider</th>
                        <th style="padding: 8px 12px;">Requests</th>
                        <th style="padding: 8px 12px;">Input Tokens</th>
                        <th style="padding: 8px 12px;">Output Tokens</th>
                        <th style="padding: 8px 12px;">Total Tokens</th>
                        <th style="padding: 8px 12px;">Cost</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each usageBreakdown as row}
                        <tr style="border-bottom: 1px solid var(--border);">
                          <td style="padding: 8px 12px;"><strong>{row.model}</strong> <span class="muted" style="font-size:11px;">{row.provider}</span></td>
                          <td style="padding: 8px 12px;">{row.requests.toLocaleString()}</td>
                          <td style="padding: 8px 12px;">{row.inputTokens == null ? '—' : row.inputTokens.toLocaleString()}</td>
                          <td style="padding: 8px 12px;">{row.outputTokens == null ? '—' : row.outputTokens.toLocaleString()}</td>
                          <td style="padding: 8px 12px;">{row.totalTokens == null ? '—' : row.totalTokens.toLocaleString()}</td>
                          <td style="padding: 8px 12px; color: var(--mint);">{row.cost == null ? '—' : `$${row.cost.toFixed(3)}`}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {:else if usageTab === 'details'}
            <div class="card">
              <div class="card-header"><h3>Request Details</h3></div>
              {#if usageDetails.length === 0}
                <p class="muted" style="padding: 16px;">No persisted request records in this period.</p>
              {:else}
                <div style="overflow-x: auto;">
                  <table style="width: 100%; border-collapse: collapse; font-size: 13px;">
                    <thead><tr><th>Time</th><th>Provider</th><th>Model</th><th>Input</th><th>Output</th><th>Cost</th><th>Status</th></tr></thead>
                    <tbody>
                      {#each usageDetails as request}
                        <tr>
                          <td>{request.timestamp}</td><td>{request.provider || 'Unknown'}</td><td>{request.model || 'Unknown'}</td>
                          <td>{request.promptTokens == null ? '—' : request.promptTokens.toLocaleString()}</td><td>{request.completionTokens == null ? '—' : request.completionTokens.toLocaleString()}</td>
                          <td>{request.cost == null ? '—' : `$${request.cost.toFixed(3)}`}</td><td>{request.status || 'unknown'}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
              {#if usageDetailsPagination.totalPages > 1}
                <div class="quota-toolbar" aria-label="Request page controls">
                  <span class="muted">Page {usageDetailsPagination.page} of {usageDetailsPagination.totalPages} ({usageDetailsPagination.totalItems} requests)</span>
                  <div class="quota-filters">
                    <button class="sm" disabled={!usageDetailsPagination.hasPrev} onclick={() => { usageDetailsPage -= 1; loadUsageDetails(); }}>Previous</button>
                    <button class="sm" disabled={!usageDetailsPagination.hasNext} onclick={() => { usageDetailsPage += 1; loadUsageDetails(); }}>Next</button>
                  </div>
                </div>
              {/if}
            </div>
          {:else if usageTab === 'logs'}
            <div class="card">
              <div class="card-header"><h3>Gateway Logs</h3></div>
              <div style="background: #000; border-radius: 8px; padding: 14px; font-family: 'JetBrains Mono', monospace; font-size: 12px; max-height: 400px; overflow-y: auto;">
                {#if usageLogs.length === 0}
                  <div class="muted">No persisted request logs in this period.</div>
                {/if}
                {#each usageLogs as log}
                  <div style="color: #94a3b8;">{log}</div>
                {/each}
              </div>
            </div>
          {/if}

        <!-- VIEW 6: QUOTA TRACKER -->
        {:else if activeView === 'quota'}
          <div class="quota-toolbar">
            <div class="quota-filters">
              <select bind:value={quotaProviderFilter} aria-label="Filter quotas by provider">
                <option value="all">All providers</option>
                {#each [...new Set(quotaItems.map(item => item.provider))] as provider}
                  <option value={provider}>{provider}</option>
                {/each}
              </select>
              <button class="period-pill" class:active={quotaFilter === 'all'} onclick={() => quotaFilter = 'all'}>All</button>
              <button class="period-pill" class:active={quotaFilter === 'active'} onclick={() => quotaFilter = 'active'}>Active</button>
            </div>
            <div class="quota-refresh">
              <span class="muted" aria-live="polite">{quotaLoading ? 'Refreshing quotas…' : quotaLastUpdated ? `Updated ${quotaLastUpdated}` : 'Not refreshed yet'}</span>
              <button class="ghost sm" onclick={loadQuota} aria-label="Refresh quota" disabled={quotaLoading}>
                <span class="material-symbols-outlined" style="font-size:18px;">refresh</span>
              </button>
            </div>
          </div>
          {#if quotaError}
            <p class="notice-banner warn" role="alert">{quotaError}</p>
          {/if}
          {#if quotaLoading && quotaItems.length === 0}
            <p class="muted" role="status">Loading saved provider quotas…</p>
          {:else if quotaItems.length === 0}
            <p class="muted" style="padding: 24px 8px;">No saved provider connections. Add a connection to check its quota.</p>
          {:else}
            <div class="grid quota-grid">
              {#each quotaItems.filter(item => (quotaProviderFilter === 'all' || item.provider === quotaProviderFilter) && (quotaFilter !== 'active' || item.is_active)) as item (item.id)}
                <article class="card quota-card">
                  <header class="quota-account">
                    <div>
                      <strong>{item.email || item.name || item.provider}</strong>
                      <div class="muted">{item.provider}{item.is_active ? ' · Enabled' : ' · Disabled'}</div>
                    </div>
                    {#if item.plan}<span class="badge">{item.plan}</span>{/if}
                  </header>
                  {#if item.message}
                    <p class="quota-message" role="status">{item.message}</p>
                  {/if}
                  {#if Object.entries(item.quotas || {}).length}
                    <div class="quota-values">
                      {#each Object.entries(item.quotas) as [key, quota]}
                        <section class="quota-row">
                          <div class="quota-row-title">
                            <strong>{quota.displayName || key}</strong>
                            {#if quota.remainingPercentage != null}
                              <span>{quota.remainingPercentage.toFixed(1)}% remaining</span>
                            {/if}
                          </div>
                          {#if quota.resetAt}
                            <div class="muted">Resets <time datetime={quota.resetAt}>{new Date(quota.resetAt).toLocaleString()}</time></div>
                          {/if}
                        </section>
                      {/each}
                    </div>
                  {:else if !item.message}
                    <p class="muted">Provider returned no quota values.</p>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        <!-- VIEW 7: TOKEN SAVER -->
        {:else if activeView === 'token-saver'}
          <div class="card">
            <div class="card-header">
              <div style="display: flex; align-items: center; gap: 10px;">
                <span class="material-symbols-outlined" style="color: #f97316; font-size: 22px;">bolt</span>
                <div>
                  <h3 style="font-size: 16px;">Token Saver</h3>
                  <p class="muted" style="margin: 2px 0 0;">Compress prompts and outputs to save tokens</p>
                </div>
              </div>
            </div>

            <div style="display: flex; flex-direction: column; gap: 14px;">
              <div class="connection-item">
                <div>
                  <div style="font-weight: 600;">Compress tool output (RTK)</div>
                  <div class="muted" style="font-size: 12px;">git/grep/ls/tree/logs: 60-90% fewer input tokens</div>
                </div>
                <label class="switch">
                  <input type="checkbox" checked={settings.rtkEnabled} onchange={(e) => updateTokenSaver({ rtk: e.target.checked })} />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="connection-item">
                <div style="display: flex; align-items: center; gap: 10px; flex: 1;">
                  <div>
                    <div style="font-weight: 600;">Compress context (Headroom)</div>
                    <div class="muted" style="font-size: 12px;">Compress prompts via /v1/compress before routing to the model</div>
                  </div>
                </div>
                <div style="display:flex; align-items:center; gap:10px;">
                  <span class="badge healthy">Running</span>
                  <a href="http://127.0.0.1:8787" target="_blank" rel="noopener noreferrer" style="font-size:12px; color:var(--mint);">Manage</a>
                  <label class="switch">
                    <input type="checkbox" checked={settings.headroomEnabled} onchange={(e) => updateTokenSaver({ headroom: e.target.checked })} />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>

              <div class="connection-item">
                <div>
                  <div style="font-weight: 600;">Compress LLM output (Caveman)</div>
                  <div class="muted" style="font-size: 12px;">Terse-style system prompt: ~65% fewer output tokens (up to 87%)</div>
                </div>
                <label class="switch">
                  <input type="checkbox" checked={!!settings.cavemanLevel && settings.cavemanLevel !== 'off'} onchange={(e) => updateTokenSaver({ caveman: e.target.checked })} />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="connection-item">
                <div>
                  <div style="font-weight: 600;">Lazy senior dev (Ponytail)</div>
                  <div class="muted" style="font-size: 12px;">Bias the model toward minimal code: YAGNI, reuse stdlib, deletion over addition</div>
                </div>
                <label class="switch">
                  <input type="checkbox" checked={!!settings.ponytailLevel && settings.ponytailLevel !== 'off'} onchange={(e) => updateTokenSaver({ ponytail: e.target.checked })} />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="connection-item">
                <div>
                  <div style="font-weight: 600;">Custom system prompt</div>
                  <div class="muted" style="font-size: 12px;">Appends your own instruction to every request, independently of the savers above</div>
                </div>
                <label class="switch">
                  <input type="checkbox" />
                  <span class="slider"></span>
                </label>
              </div>
            </div>
          </div>

        <!-- VIEW 8: CLI TOOLS -->
        {:else if activeView === 'cli-tools'}
          <div class="grid" style="grid-template-columns: repeat(3, 1fr);">
            {#each (Array.isArray(cliStatuses) ? cliStatuses : Object.entries(cliStatuses).map(([key, tool]) => ({key, ...tool}))) as tool (tool.key)}
              <div class="cli-tool-card" onclick={() => selectedCliTool = selectedCliTool?.key === tool.key ? null : tool} role="button" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter') selectedCliTool = selectedCliTool?.key === tool.key ? null : tool; }}>
                <div style="width: 32px; height: 32px; border-radius: 8px; background: var(--panel-2); border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; flex-shrink: 0;">
                  <span class="material-symbols-outlined" style="font-size: 18px; color: var(--mint);">{tool.icon || 'terminal'}</span>
                </div>
                <div style="flex: 1; min-width: 0;">
                  <div style="font-weight: 600; font-size: 13.5px;">{tool.name || tool.key}</div>
                  <span class="status-pill {tool.status || (tool.installed ? 'connected' : 'not-installed')}">{tool.status === 'connected' ? 'Connected' : tool.status === 'guide' ? 'Guide' : tool.status === 'not-configured' ? 'Not configured' : tool.installed ? 'Connected' : 'Not installed'}</span>
                </div>
                <span class="material-symbols-outlined" style="font-size: 18px; color: var(--muted);">chevron_right</span>
              </div>
            {/each}
          </div>

          {#if selectedCliTool}
            <div class="card" style="margin-top: 16px;">
              <div class="card-header">
                <h3>{selectedCliTool.name || selectedCliTool.key} Configuration</h3>
                <button class="ghost sm" onclick={() => selectedCliTool = null}><span class="material-symbols-outlined">close</span></button>
              </div>
              <div style="display: flex; flex-direction: column; gap: 14px;">
                <div>
                  <div class="muted" style="font-size: 12px; margin-bottom: 4px;">Status</div>
                  <span class="status-pill {selectedCliTool.status || (selectedCliTool.installed ? 'connected' : 'not-installed')}">{selectedCliTool.message}</span>
                </div>
                {#if selectedCliTool.configCommand}
                  <div>
                    <div class="muted" style="font-size: 12px; margin-bottom: 4px;">Configuration Command</div>
                    <div style="display: flex; gap: 8px; align-items: center;">
                      <code style="flex: 1; padding: 8px 12px; font-size: 12px; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{selectedCliTool.configCommand}</code>
                      <button class="sm" onclick={() => copyText(selectedCliTool.configCommand)}><span class="material-symbols-outlined" style="font-size:16px;">content_copy</span></button>
                    </div>
                  </div>
                {/if}
                {#if selectedCliTool.envVars && Object.keys(selectedCliTool.envVars).length > 0}
                  <div>
                    <div class="muted" style="font-size: 12px; margin-bottom: 4px;">Environment Variables</div>
                    {#each Object.entries(selectedCliTool.envVars) as [k, v]}
                      <div style="display: flex; gap: 8px; align-items: center; margin-bottom: 6px;">
                        <code style="flex: 1; padding: 6px 10px; font-size: 11.5px; background: var(--panel-2); border: 1px solid var(--border); border-radius: 4px;">export {k}="{v}"</code>
                        <button class="ghost sm" onclick={() => copyText(`export ${k}="${v}"`)}><span class="material-symbols-outlined" style="font-size:14px;">content_copy</span></button>
                      </div>
                    {/each}
                  </div>
                {/if}
                {#if selectedCliTool.installGuide}
                  <div>
                    <div class="muted" style="font-size: 12px; margin-bottom: 4px;">Install Guide</div>
                    <div style="display: flex; gap: 8px; align-items: center;">
                      <code style="flex: 1; padding: 8px 12px; font-size: 12px; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px;">{selectedCliTool.installGuide}</code>
                      <button class="sm" onclick={() => copyText(selectedCliTool.installGuide)}><span class="material-symbols-outlined" style="font-size:16px;">content_copy</span></button>
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        <!-- VIEW 9: TRANSLATOR WORKBENCH -->
        {:else if activeView === 'translator'}
          <div class="card">
            <div class="card-header">
              <h3>6-Step Request &amp; Response Translation Flow</h3>
              <button class="primary sm" onclick={async () => {
                const res = await fetch('/api/translator/run', {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({
                    source_format: 'openai',
                    target_format: 'claude',
                    payload: { model: 'gpt-4o', messages: [{ role: 'user', content: 'Hello' }] }
                  })
                });
                const data = await res.json();
                showToast(`Converted in ${data.stats?.translation_time_ms || 1}ms`);
              }}>
                <span class="material-symbols-outlined" style="font-size:16px;">play_arrow</span> Run Translation
              </button>
            </div>

            <div style="display: flex; flex-direction: column; gap: 10px;">
              <div class="connection-item">
                <div><strong>1. Client Request</strong> - Native format sent by IDE / SDK</div>
                <span class="badge active">Step 1</span>
              </div>
              <div class="connection-item">
                <div><strong>2. Source Body</strong> - Decoded JSON payload with parameters</div>
                <span class="badge active">Step 2</span>
              </div>
              <div class="connection-item">
                <div><strong>3. OpenAI Intermediate</strong> - Canonical intermediate representation</div>
                <span class="badge active">Step 3</span>
              </div>
              <div class="connection-item">
                <div><strong>4. Upstream Request</strong> - Adapted for upstream provider API</div>
                <span class="badge active">Step 4</span>
              </div>
              <div class="connection-item">
                <div><strong>5. Upstream Response</strong> - Raw streaming chunks or JSON</div>
                <span class="badge active">Step 5</span>
              </div>
              <div class="connection-item">
                <div><strong>6. Client Response</strong> - Transformed response sent to client</div>
                <span class="badge active">Step 6</span>
              </div>
            </div>
          </div>

        <!-- VIEW 10: SKILLS (INJECTED FROM HTTPS://GITHUB.COM/TOPICS/AGENT-SKILLS) -->
        {:else if activeView === 'skills'}
          <div style="display: flex; flex-direction: column; gap: 14px;">
            <div style="display: flex; gap: 12px; align-items: center; flex-wrap: wrap;">
              <div style="display: flex; gap: 4px; background: var(--panel-2); border-radius: 6px; padding: 2px;">
                <button class="sm" style="border: none; border-radius: 4px; {skillSource === 'github' ? 'background: var(--mint); color: #000; font-weight: 600;' : 'background: transparent; color: var(--muted);'}" onclick={() => { skillSource = 'github'; if (githubResults.length === 0) searchGitHubSkills('agent-skills'); }}>GitHub Live</button>
                <button class="sm" style="border: none; border-radius: 4px; {skillSource === 'curated' ? 'background: var(--mint); color: #000; font-weight: 600;' : 'background: transparent; color: var(--muted);'}" onclick={() => skillSource = 'curated'}>Curated ({agentSkillsList.length})</button>
              </div>

              {#if skillSource === 'github'}
                <div class="search-input-wrap" style="flex: 1; min-width: 280px;">
                  <span class="material-symbols-outlined">search</span>
                  <input
                    type="text"
                    placeholder="Search 15,000+ agent skills on GitHub..."
                    bind:value={skillSearch}
                    onkeydown={(e) => { if (e.key === 'Enter') searchGitHubSkills(skillSearch); }}
                  />
                </div>
                <button class="sm primary" onclick={() => searchGitHubSkills(skillSearch)} disabled={githubLoading}>
                  {githubLoading ? 'Searching...' : 'Search GitHub'}
                </button>
              {:else}
                <div class="search-input-wrap" style="flex: 1; min-width: 280px;">
                  <span class="material-symbols-outlined">search</span>
                  <input
                    type="text"
                    placeholder="Filter curated agent skills..."
                    bind:value={skillSearch}
                  />
                </div>
              {/if}

              <div style="width: 240px;">
                <select bind:value={skillCategory}>
                  <option value="all">All Categories</option>
                  {#each skillCategories.filter(c => c !== 'all') as cat}
                    <option value={cat}>{cat}</option>
                  {/each}
                </select>
              </div>
            </div>

            <div class="section-title-bar">
              <h2>
                <span class="material-symbols-outlined" style="color:var(--mint);">extension</span>
                {skillSource === 'curated' ? 'Curated' : 'GitHub Live'} Agent Skills ({filteredSkills.length}{skillSource === 'github' ? ` / ${githubTotal.toLocaleString()}` : ''})
              </h2>
              <a href="https://github.com/topics/agent-skills" target="_blank" rel="noopener noreferrer" class="btn ghost sm">
                <span class="material-symbols-outlined" style="font-size: 16px;">open_in_new</span>
                github.com/topics/agent-skills
              </a>
            </div>

            {#if githubLoading}
              <div style="text-align: center; padding: 40px; color: var(--muted);">
                <span class="material-symbols-outlined" style="font-size: 32px; animation: spin 1s linear infinite;">progress_activity</span>
                <p>Searching GitHub repositories...</p>
              </div>
            {/if}

            <div class="grid" style="grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));">
              {#each filteredSkills as skill (skill.id)}
                <div class="card" style="padding: 18px; display: flex; flex-direction: column; justify-content: space-between; gap: 12px;">
                  <div>
                    <div style="display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; margin-bottom: 6px;">
                      <div style="font-weight: 700; font-size: 14px; color: var(--text-strong);">{skill.name}</div>
                      <span class="badge" style="background: rgba(245, 158, 11, 0.12); color: #fbbf24; border-color: rgba(245, 158, 11, 0.3); font-size: 11px; white-space: nowrap;">
                        ★ {(skill.stars || 0).toLocaleString()}
                      </span>
                    </div>

                    <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 8px;">
                      <span class="badge active" style="font-size: 10.5px;">{skill.category}</span>
                      <a href={skill.url} target="_blank" rel="noopener noreferrer" style="font-size: 11.5px; color: var(--muted); text-decoration: none; display: inline-flex; align-items: center; gap: 3px;">
                        <span>{skill.repo}</span>
                        <span class="material-symbols-outlined" style="font-size: 13px;">open_in_new</span>
                      </a>
                    </div>

                    <p class="muted" style="font-size: 12.5px; line-height: 1.5; margin-bottom: 12px;">
                      {skill.description || 'No description available.'}
                    </p>

                    <div style="display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 12px;">
                      {#each (skill.tags || []) as tag}
                        <span style="font-size: 10.5px; padding: 1px 6px; border-radius: 4px; background: var(--panel-2); color: var(--muted);">
                          #{tag}
                        </span>
                      {/each}
                    </div>
                  </div>

                  <div style="padding-top: 10px; border-top: 1px solid var(--border); display: flex; align-items: center; gap: 8px;">
                    <div style="flex: 1; background: var(--panel-2); border: 1px solid var(--border-light); border-radius: 4px; padding: 5px 8px; font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--mint-bright); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                      {skill.installCommand}
                    </div>
                    <button class="sm primary" onclick={() => copyText(skill.installCommand)} title="Copy install command" aria-label="Copy install command">
                      <span class="material-symbols-outlined" style="font-size: 15px;">content_copy</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>

            {#if skillSource === 'github' && githubResults.length < githubTotal && !githubLoading}
              <div style="text-align: center; padding: 16px;">
                <button class="primary" onclick={() => searchGitHubSkills(githubQuery, githubPage + 1)}>
                  Load More ({githubResults.length} / {githubTotal.toLocaleString()})
                </button>
              </div>
            {/if}
          </div>

        <!-- VIEW 11: MEDIA PROVIDERS -->
        {:else if activeView === 'media'}
          <div class="card">
            <div class="card-header">
              <h3>Media Providers Hub</h3>
              <span class="badge active">Unified Media Router</span>
            </div>
            <p class="muted">
              Configure specialized non-LLM providers including audio, image, video, search and scraping adapters.
            </p>
          </div>

        <!-- VIEW 12: PROXY POOLS -->
        {:else if activeView === 'proxy-pools'}
          <div class="card">
            <div class="card-header">
              <h3>Configured Proxy Pools ({proxyPools.length})</h3>
              <div style="display:flex;gap:8px;">
                <button class="sm" onclick={() => showToast('Cloudflare relay deploy triggered')}>
                  <span class="material-symbols-outlined" style="font-size:16px;">cloud_upload</span> Deploy Relay
                </button>
                <button class="primary sm" onclick={() => { modalType = 'add-proxy'; modalData = {}; }}>
                  <span class="material-symbols-outlined" style="font-size:16px;">add</span> Add Proxy Pool
                </button>
              </div>
            </div>

            <div class="muted" style="text-align:center; padding: 24px;">
              No proxy pools configured. Outbound requests connect directly to upstream endpoints.
            </div>
          </div>

        <!-- VIEW 13: CONSOLE LOG -->
        {:else if activeView === 'console-log'}
          <div class="card">
            <div class="card-header">
              <h3>Live Gateway Activity Log</h3>
              <button class="sm" onclick={() => { consoleLogs = []; showToast('Logs cleared'); }}>
                <span class="material-symbols-outlined" style="font-size: 16px;">clear_all</span> Clear
              </button>
            </div>

            <div style="background: #000; border-radius: 8px; padding: 14px; font-family: 'JetBrains Mono', monospace; font-size: 12px; max-height: 400px; overflow-y: auto;">
              <div style="color: #10b981;">[9router-rust] Native engine initialized on port 20129</div>
              <div style="color: #60a5fa;">[headroom] Context compression pipeline active</div>
              <div style="color: #94a3b8;">[sqlite] Storage database connected</div>
              <div style="color: #f59e0b;">[router] Ready for incoming proxy requests</div>
            </div>
          </div>

        <!-- VIEW 14: SETTINGS -->
        {:else if activeView === 'settings'}
          <div class="card">
            <div class="card-header">
              <h3>Gateway Configuration</h3>
              <button class="primary sm" onclick={async () => {
                await fetch('/api/settings', {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify(settings)
                });
                showToast('Settings saved');
              }}>
                Save Settings
              </button>
            </div>

            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
              <div>
                <label style="font-size: 12px; color: var(--muted);">Listening Port</label>
                <input type="number" bind:value={settings.port} style="margin-top: 4px;" />
              </div>

              <div>
                <label style="font-size: 12px; color: var(--muted);">Require Login</label>
                <div style="margin-top: 8px;">
                  <label class="switch">
                    <input type="checkbox" bind:checked={settings.requireLogin} />
                    <span class="slider"></span>
                  </label>
                </div>
              </div>
            </div>
          </div>

          <div class="card">
            <div class="card-header">
              <h3>Database &amp; Maintenance</h3>
            </div>
            <div style="display: flex; gap: 10px;">
              <button class="sm" onclick={() => showToast('Database backup downloaded')}>
                <span class="material-symbols-outlined" style="font-size:16px;">download</span> Download Backup
              </button>
              <button class="sm" onclick={() => showToast('Database optimized')}>
                <span class="material-symbols-outlined" style="font-size:16px;">cleaning_services</span> Optimize DB
              </button>
            </div>
          </div>
        {/if}
      </main>
    </div>
  </div>
{/if}

<!-- MODALS -->
{#if modalType === 'add-connection' && modalData.mode === 'oauth'}
  <!-- 100% PARITY OAUTH CONNECTION MODAL (EXACT MATCH FOR IMAGE #1) -->
  <div class="modal" onclick={() => modalType = null} role="presentation">
    <div class="modal-card oauth-modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="oauth-popup-text">
      <!-- WAITING FOR POPUP AUTHORIZATION (CLICKABLE BUTTON TO LAUNCH/RE-FOCUS POPUP) -->
      <a
        href={modalData.authUrl}
        target="_blank"
        rel="noopener noreferrer"
        class="oauth-auth-box oauth-auth-box-link"
        title="Open authorization page in browser"
        aria-label="Open authorization page"
      >
        <div class="oauth-spinner" aria-hidden="true"></div>
        <span id="oauth-popup-text" class="oauth-wait-text">Waiting for popup authorization...</span>
        <span class="material-symbols-outlined" style="margin-left: auto; font-size: 16px; color: var(--muted-dark);">open_in_new</span>
      </a>

      <!-- OR PASTE CALLBACK URL MANUALLY DIVIDER -->
      <div class="oauth-divider">
        <div class="oauth-divider-line"></div>
        <span class="oauth-divider-text">OR PASTE CALLBACK URL MANUALLY</span>
        <div class="oauth-divider-line"></div>
      </div>

      <!-- STEP 1: OPEN THIS URL IN YOUR BROWSER -->
      <div class="oauth-step">
        <label for="oauth-step1-input" class="oauth-step-label">Step 1: Open this URL in your browser</label>
        <div class="oauth-url-row">
          <input
            id="oauth-step1-input"
            type="text"
            readonly
            value={modalData.authUrl}
            class="oauth-mono-input"
            onclick={(e) => { e.currentTarget.select(); }}
          />
          <a
            href={modalData.authUrl}
            target="_blank"
            rel="noopener noreferrer"
            class="oauth-copy-btn"
            title="Open in browser"
          >
            <span class="material-symbols-outlined" style="font-size: 16px;">open_in_new</span>
            <span>Open</span>
          </a>
          <button type="button" class="oauth-copy-btn" onclick={() => copyAuthUrl(modalData.authUrl)} title="Copy URL">
            <span class="material-symbols-outlined" style="font-size: 16px;">
              {modalData.copied ? 'check' : 'content_copy'}
            </span>
            <span>{modalData.copied ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </div>

      <!-- STEP 2: PASTE THE CALLBACK URL HERE -->
      <div class="oauth-step">
        <label for="oauth-step2-input" class="oauth-step-label">Step 2: Paste the callback URL here</label>
        <p class="oauth-step-desc">After authorization, copy the full URL from your browser.</p>
        <input
          id="oauth-step2-input"
          type="text"
          placeholder="http://localhost:20128/callback?code=..."
          bind:value={modalData.callbackUrl}
          class="oauth-mono-input"
        />
      </div>

      <!-- ACTIONS: CONNECT & CANCEL -->
      <div class="oauth-actions-row">
        <button
          type="button"
          class="oauth-btn-connect {modalData.callbackUrl?.trim() ? 'primary' : 'disabled'}"
          disabled={!modalData.callbackUrl?.trim() || modalData.isConnecting}
          onclick={handleConnectOAuth}
        >
          {modalData.isConnecting ? 'Connecting...' : 'Connect'}
        </button>
        <button type="button" class="oauth-btn-cancel ghost" onclick={() => modalType = null}>
          Cancel
        </button>
      </div>

      {#if selectedProvider.category !== 'oauth'}
        <div style="margin-top: 8px; text-align: center;">
          <button type="button" class="ghost sm" onclick={() => modalData.mode = 'apikey'}>
            Or enter API Key directly
          </button>
        </div>
      {/if}
    </div>
  </div>
{:else if modalType}
  <div class="modal" onclick={() => modalType = null} role="presentation">
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <div class="modal-header">
        <h3 style="font-size: 16px;">
          {#if modalType === 'custom-provider'}
            Add Custom Provider
          {:else if modalType === 'add-connection'}
            Add Connection for {selectedProvider.name}
          {:else if modalType === 'add-model'}
            Add Custom Model
          {:else if modalType === 'create-key'}
            Create API Key
          {:else if modalType === 'create-combo'}
            Create Model Combo
          {:else if modalType === 'add-proxy'}
            Add Proxy Pool
          {/if}
        </h3>
        <button class="ghost sm" onclick={() => modalType = null} aria-label="Close dialog">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>

      <div class="modal-body">
        {#if modalType === 'custom-provider'}
          <div>
            <label style="font-size: 12px; color: var(--muted);">Provider Display Name</label>
            <input type="text" placeholder="e.g. My Custom LLM" bind:value={modalData.name} />
          </div>
          <div>
            <label style="font-size: 12px; color: var(--muted);">Provider ID</label>
            <input type="text" placeholder="e.g. my-custom-llm" bind:value={modalData.id} />
          </div>
          <div>
            <label style="font-size: 12px; color: var(--muted);">Base URL</label>
            <input type="text" placeholder="https://api.example.com/v1" bind:value={modalData.baseUrl} />
          </div>
          <div>
            <label style="font-size: 12px; color: var(--muted);">API Key</label>
            <input type="password" placeholder="sk-..." bind:value={modalData.apiKey} />
          </div>
        {:else if modalType === 'add-connection'}
          <div>
            <label style="font-size: 12px; color: var(--muted);">Connection Name / Email</label>
            <input type="text" placeholder="Account identifier" bind:value={modalData.name} />
          </div>
          <div>
            <label style="font-size: 12px; color: var(--muted);">API Key / Token</label>
            <input type="password" placeholder="API key or OAuth token" bind:value={modalData.apiKey} />
          </div>
          <div style="margin-top: 6px;">
            <button type="button" class="ghost sm" onclick={() => modalData.mode = 'oauth'}>
              Or connect via browser OAuth popup
            </button>
          </div>
        {:else if modalType === 'add-model'}
          <div>
            <label style="font-size: 12px; color: var(--muted);">Model ID</label>
            <input type="text" placeholder="e.g. gpt-4o-custom" bind:value={modalData.modelId} />
          </div>
          <div>
            <label style="font-size: 12px; color: var(--muted);">Model Display Name</label>
            <input type="text" placeholder="e.g. GPT-4o Custom" bind:value={modalData.modelName} />
          </div>
        {:else if modalType === 'create-key'}
          <div>
            <label style="font-size: 12px; color: var(--muted);">Key Name</label>
            <input type="text" placeholder="e.g. Cursor IDE Key" bind:value={modalData.name} />
          </div>
        {:else if modalType === 'create-combo'}
          <div>
            <label style="font-size: 12px; color: var(--muted);">Combo Name</label>
            <input type="text" placeholder="e.g. auto-fast" bind:value={modalData.name} />
          </div>
          <div>
            <label style="font-size: 12px; color: var(--muted);">Model Chain (comma separated)</label>
            <input type="text" placeholder="antigravity/claude-sonnet-4, openai/gpt-4o" bind:value={modalData.models} />
          </div>
        {:else if modalType === 'add-proxy'}
          <div>
            <label style="font-size: 12px; color: var(--muted);">Proxy URL</label>
            <input type="text" placeholder="socks5://127.0.0.1:1080" bind:value={modalData.url} />
          </div>
        {/if}
      </div>

      <div class="modal-actions">
        <button class="ghost sm" onclick={() => modalType = null}>Cancel</button>
        <button class="primary sm" onclick={async () => {
          if (modalType === 'custom-provider') {
            await fetch('/api/providers', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                id: modalData.id || `custom-${Date.now()}`,
                provider: modalData.id || 'custom',
                name: modalData.name || 'Custom Provider',
                authType: 'apikey',
                isActive: true,
                baseUrl: modalData.baseUrl,
                apiKey: modalData.apiKey
              })
            });
            loadProviders();
            showToast('Custom provider added');
          } else if (modalType === 'add-connection') {
            await fetch('/api/providers', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                provider: selectedProviderId,
                name: modalData.name,
                authType: 'apikey',
                apiKey: modalData.apiKey,
                isActive: true
              })
            });
            loadProviders();
            showToast('Connection added');
          } else if (modalType === 'add-model') {
            addModelToProvider(modalData.modelId, modalData.modelName);
          } else if (modalType === 'create-key') {
            await fetch('/api/keys', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ name: modalData.name })
            });
            loadKeys();
            showToast('API key created');
          } else if (modalType === 'create-combo') {
            await fetch('/api/combos', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                id: `combo-${Date.now()}`,
                name: modalData.name,
                models: modalData.models
              })
            });
            loadCombos();
            showToast('Combo created');
          }
          modalType = null;
        }}>
          Save
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .toast-popup {
    position: fixed;
    bottom: 24px;
    right: 24px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 12px 18px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.5);
    display: flex;
    align-items: center;
    gap: 10px;
    z-index: 9999;
    font-size: 13px;
    font-weight: 500;
  }
  .toast-popup.success {
    border-color: rgba(16, 185, 129, 0.4);
    color: var(--mint);
  }
  .toast-popup.error {
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }
</style>
