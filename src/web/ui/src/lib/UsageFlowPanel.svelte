<script>
  import { Activity, ArrowUpRight, Clock3, Network } from '@lucide/svelte';

  export let providers = [];
  export let requests = [];
  export let activeProviders = [];
  export let period = 'all';
  export let error = '';
  export let loading = false;
  export let routesLoading = false;

  $: graphHeight = Math.max(216, providers.length * 56 + 16);
  $: sourceY = graphHeight / 2;
  $: activeProviderLabel = [...new Set(activeProviders)].join(', ');
</script>

<section class="mb-4 grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1.35fr)_minmax(280px,0.85fr)]" aria-label="Gateway request activity">
  <article class="min-w-0 rounded-lg border border-line bg-surface p-4">
    <header class="mb-3 flex flex-wrap items-center justify-between gap-2">
      <div>
        <h2 class="flex items-center gap-2 text-sm font-semibold text-copy"><Network size={16} aria-hidden="true" /> Provider routes</h2>
        <p class="mt-1 text-xs text-copy-muted">Recorded requests routed from 9Router during {period}.</p>
      </div>
      {#if activeProviders.length}
        <span class="inline-flex items-center gap-1.5 rounded-md border border-accent/40 bg-accent/10 px-2 py-1 text-xs font-medium text-accent" role="status" aria-live="polite">
          <Activity size={14} aria-hidden="true" /> New request: {activeProviderLabel}
        </span>
      {/if}
    </header>

    {#if providers.length}
      <div class="relative grid grid-cols-[112px_minmax(0,1fr)] gap-3 sm:grid-cols-[150px_minmax(0,1fr)]" style={`min-height:${graphHeight}px`}>
        <svg class="pointer-events-none absolute inset-0 h-full w-full" viewBox={`0 0 720 ${graphHeight}`} preserveAspectRatio="none" role="img" aria-label="Recorded provider routes from 9Router">
          <title>Recorded provider routes for {period}</title>
          {#each providers as item, index}
            {@const rowY = 36 + index * 56}
            <path d={`M 150 ${sourceY} C 220 ${sourceY}, 150 ${rowY}, 260 ${rowY}`} fill="none" stroke={activeProviders.includes(item.provider) ? 'var(--mint)' : 'var(--border-light)'} stroke-width={activeProviders.includes(item.provider) ? 2.5 : 1.5} stroke-dasharray={activeProviders.includes(item.provider) ? '8 8' : '4 6'} class={activeProviders.includes(item.provider) ? 'animate-route-flow motion-reduce:animate-none' : ''} />
          {/each}
        </svg>

        <div class="relative z-10 flex items-center">
          <div class="w-full rounded-md border border-accent/50 bg-surface px-2 py-3 text-center sm:px-3">
            <Network class="mx-auto mb-1 text-accent" size={20} aria-hidden="true" />
            <strong class="block text-sm text-copy">9Router</strong>
            <span class="text-[11px] text-copy-muted">Gateway</span>
          </div>
        </div>

        <div class="relative z-10 flex min-w-0 flex-col gap-3 py-[14px]">
          {#each providers as item}
            <div class="ml-[36%] flex h-11 min-w-0 items-center justify-between gap-2 rounded-md border border-line bg-surface px-2.5 sm:px-3">
              <span class="min-w-0 truncate text-xs font-medium text-copy" title={item.provider}>{item.provider}</span>
              <span class="shrink-0 text-[11px] text-copy-muted">{item.requests.toLocaleString()} req</span>
            </div>
          {/each}
        </div>
      </div>
      {:else if routesLoading}
      <p class="rounded-md border border-dashed border-line px-4 py-8 text-center text-sm text-copy-muted" role="status">Loading recorded provider routes for {period}…</p>
      {:else}
      <p class="rounded-md border border-dashed border-line px-4 py-8 text-center text-sm text-copy-muted" role="status">No recorded provider routes for {period}. Requests appear here after the gateway records them.</p>
    {/if}
  </article>

  <article class="min-w-0 rounded-lg border border-line bg-surface p-4">
    <header class="mb-3 flex items-center justify-between gap-2">
      <div>
        <h2 class="flex items-center gap-2 text-sm font-semibold text-copy"><Activity size={16} aria-hidden="true" /> Recent requests</h2>
        <p class="mt-1 text-xs text-copy-muted">Latest persisted gateway records</p>
      </div>
      <ArrowUpRight size={16} class="text-copy-muted" aria-hidden="true" />
    </header>

    {#if error}
      <p class="mb-3 rounded-md border border-warning/40 bg-warning/10 p-2.5 text-xs text-warning" role="alert">Request refresh failed: {error}. Showing the last records received.</p>
    {/if}
    {#if requests.length}
      <ol class="divide-y divide-line">
        {#each requests.slice(0, 6) as request}
          <li class="flex min-w-0 items-start justify-between gap-3 py-3 first:pt-1 last:pb-1">
            <div class="min-w-0">
              <p class="truncate text-xs font-medium text-copy" title={request.model || 'Unknown model'}>{request.model || 'Unknown model'}</p>
              <p class="mt-1 truncate text-[11px] text-copy-muted" title={request.provider || 'Unknown provider'}>{request.provider || 'Unknown provider'} · {request.status || 'unknown'}</p>
            </div>
            <time class="flex shrink-0 items-center gap-1 text-[10px] text-copy-muted" datetime={request.timestamp}>
              <Clock3 size={12} aria-hidden="true" /> {request.timestamp?.split('T')[1]?.slice(0, 8) || request.timestamp}
            </time>
          </li>
        {/each}
      </ol>
    {:else if loading}
      <p class="rounded-md border border-dashed border-line px-3 py-8 text-center text-sm text-copy-muted" role="status">Loading recent requests…</p>
    {:else if !error}
      <p class="rounded-md border border-dashed border-line px-3 py-8 text-center text-sm text-copy-muted" role="status">No persisted requests yet. Send a request through the gateway to populate this list.</p>
    {/if}
  </article>
</section>
