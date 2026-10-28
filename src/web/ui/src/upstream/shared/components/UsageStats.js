"use client";

import { useState, useEffect, useMemo, useCallback } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import Badge from "./Badge";
import Card from "./Card";
import OverviewCards from "@/app/(dashboard)/dashboard/usage/components/OverviewCards";
import UsageTable, { fmt, fmtTime } from "@/app/(dashboard)/dashboard/usage/components/UsageTable";
import dynamic from "next/dynamic";
// Keep chart libraries out of the initial bundle.
const UsageChart = dynamic(() => import("@/app/(dashboard)/dashboard/usage/components/UsageChart"), { ssr: false });
const ProviderBarChart = dynamic(() => import("@/app/(dashboard)/dashboard/usage/components/ProviderBarChart"), { ssr: false });
const TopModelsChart = dynamic(() => import("@/app/(dashboard)/dashboard/usage/components/TopModelsChart"), { ssr: false });

function timeAgo(timestamp) {
  const diff = Math.floor((Date.now() - new Date(timestamp)) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

// Auto-update time display every second without re-rendering parent
function TimeAgo({ timestamp }) {
  const [, setTick] = useState(0);
  
  useEffect(() => {
    const timer = setInterval(() => setTick(t => t + 1), 1000);
    return () => clearInterval(timer);
  }, []);
  
  return <>{timeAgo(timestamp)}</>;
}

function RecentRequests({ requests = [] }) {
  return (
    <Card className="flex min-w-0 flex-col overflow-hidden" padding="sm" style={{ height: 480 }}>
      {/* Header */}
      <div className="px-1 py-2 border-b border-border shrink-0">
        <span className="text-xs font-semibold text-text-muted uppercase tracking-wide">Recent Requests</span>
      </div>

      {!requests.length ? (
        <div className="flex-1 flex items-center justify-center text-text-muted text-sm">No requests yet.</div>
      ) : (
        <div className="flex-1 overflow-y-auto">
          <table className="w-full min-w-[300px] border-collapse text-xs">
            <thead className="sticky top-0 bg-bg z-10">
              <tr className="border-b border-border">
                <th className="py-1.5 text-left font-semibold text-text-muted w-2"></th>
                <th className="py-1.5 text-left font-semibold text-text-muted">Model</th>
                <th className="py-1.5 text-right font-semibold text-text-muted whitespace-nowrap">In / Out</th>
                <th className="py-1.5 text-right font-semibold text-text-muted">When</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border/50">
              {requests.map((r, i) => {
                const ok = !r.status || r.status === "ok" || r.status === "success";
                return (
                  <tr key={i} className="hover:bg-bg-subtle transition-colors">
                    <td className="py-1.5">
                      <span className={`block w-1.5 h-1.5 rounded-full ${ok ? "bg-success" : "bg-error"}`} />
                    </td>
                    <td className="py-1.5 font-mono truncate max-w-[120px]" title={r.model}>{r.model}</td>
                    <td className="py-1.5 text-right whitespace-nowrap">
                      <span className="text-primary">{r.promptTokens == null ? "Unavailable" : fmt(r.promptTokens)}↑</span>
                      {" "}
                      <span className="text-success">{r.completionTokens == null ? "Unavailable" : fmt(r.completionTokens)}↓</span>
                    </td>
                    <td className="py-1.5 text-right text-text-muted whitespace-nowrap"><TimeAgo timestamp={r.timestamp} /></td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </Card>
  );
}

function sortData(dataMap, sortBy, sortOrder) {
  return Object.entries(dataMap || {})
    .map(([key, data]) => ({
      ...data,
      key,
      totalTokens: data.promptTokens == null || data.completionTokens == null ? null : data.promptTokens + data.completionTokens,
      totalCost: data.cost,
    }))
    .sort((a, b) => {
      let valA = a[sortBy];
      let valB = b[sortBy];
      if (typeof valA === "string") valA = valA.toLowerCase();
      if (typeof valB === "string") valB = valB.toLowerCase();
      if (valA < valB) return sortOrder === "asc" ? -1 : 1;
      if (valA > valB) return sortOrder === "asc" ? 1 : -1;
      return 0;
    });
}

function groupDataByKey(data, keyField) {
  if (!Array.isArray(data)) return [];
  const groups = {};
  data.forEach((item) => {
    const gk = item[keyField] || "Unknown";
    if (!groups[gk]) {
      groups[gk] = {
        groupKey: gk,
        summary: { requests: 0, promptTokens: 0, completionTokens: 0, totalTokens: 0, cost: 0, lastUsed: null },
        items: [],
      };
    }
    const summary = groups[gk].summary;
    summary.requests += item.requests;
    summary.promptTokens = summary.promptTokens == null || item.promptTokens == null ? null : summary.promptTokens + item.promptTokens;
    summary.completionTokens = summary.completionTokens == null || item.completionTokens == null ? null : summary.completionTokens + item.completionTokens;
    summary.totalTokens = summary.promptTokens == null || summary.completionTokens == null ? null : summary.promptTokens + summary.completionTokens;
    summary.cost = summary.cost == null || item.cost == null ? null : summary.cost + item.cost;
    if (item.lastUsed && (!summary.lastUsed || new Date(item.lastUsed) > new Date(summary.lastUsed))) summary.lastUsed = item.lastUsed;
    groups[gk].items.push(item);
  });
  return Object.values(groups);
}

const MODEL_COLUMNS = [
  { field: "rawModel", label: "Model" },
  { field: "provider", label: "Provider" },
  { field: "requests", label: "Requests", align: "right" },
  { field: "lastUsed", label: "Last Used", align: "right" },
];


const PERIODS = [
  { value: "today", label: "Today" },
  { value: "24h", label: "24h" },
  { value: "7d", label: "7D" },
  { value: "30d", label: "30D" },
  { value: "60d", label: "60D" },
  { value: "all", label: "All" },
];

export default function UsageStats({ period: periodProp, setPeriod: setPeriodProp, hidePeriodSelector = false } = {}) {
  const router = useRouter();
  const searchParams = useSearchParams();

  const sortBy = searchParams.get("sortBy") || "rawModel";
  const sortOrder = searchParams.get("sortOrder") || "asc";

  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [fetching, setFetching] = useState(false);
  const [error, setError] = useState(null);
  const [viewMode, setViewMode] = useState("costs");
  const [periodLocal, setPeriodLocal] = useState("today");
  const period = periodProp ?? periodLocal;
  const setPeriod = setPeriodProp ?? setPeriodLocal;

  useEffect(() => {
    let active = true;
    setLoading(true);
    setFetching(true);
    setStats(null);
    fetch(`/api/usage/history?period=${period}`)
      .then(async (response) => {
        if (!response.ok) throw new Error(`Usage request failed (HTTP ${response.status})`);
        const data = await response.json();
        const records = data.history || [];
        const lastUsed = new Map();
        for (const row of records) {
          const key = `${row.model || "unknown"} (${row.provider || "unknown"})`;
          if (!lastUsed.has(key) || Date.parse(row.timestamp) > Date.parse(lastUsed.get(key))) lastUsed.set(key, row.timestamp);
        }
        const byModel = Object.fromEntries((data.breakdown || []).map((row) => {
          const key = `${row.model} (${row.provider})`;
          return [key, {
            rawModel: row.model,
            provider: row.provider,
            requests: row.requests,
            promptTokens: row.inputTokens,
            completionTokens: row.outputTokens,
            cost: row.cost,
            lastUsed: lastUsed.get(key),
          }];
        }));
        if (active) setStats({ ...data.stats, byModel, recentRequests: records.slice(0, 20) });
      })
      .catch((err) => { if (active) setError(err.message); })
      .finally(() => {
        if (active) {
          setLoading(false);
          setFetching(false);
        }
      });
    return () => { active = false; };
  }, [period]);

  const toggleSort = useCallback((tableType, field) => {
    const params = new URLSearchParams(searchParams.toString());
    if (params.get("sortBy") === field) {
      params.set("sortOrder", params.get("sortOrder") === "asc" ? "desc" : "asc");
    } else {
      params.set("sortBy", field);
      params.set("sortOrder", "asc");
    }
    router.replace(`?${params.toString()}`, { scroll: false });
  }, [searchParams, router]);

  const activeTableConfig = useMemo(() => ({
    columns: MODEL_COLUMNS,
    groupedData: groupDataByKey(sortData(stats?.byModel, sortBy, sortOrder), "rawModel"),
    storageKey: "usage-stats:expanded-models",
    emptyMessage: "No usage recorded for this period.",
    renderSummaryCells: (group) => (
      <>
        <td className="px-6 py-3 text-text-muted">—</td>
        <td className="px-6 py-3 text-right">{fmt(group.summary.requests)}</td>
        <td className="px-6 py-3 text-right text-text-muted whitespace-nowrap">{fmtTime(group.summary.lastUsed)}</td>
      </>
    ),
    renderDetailCells: (item) => (
      <>
        <td className="px-6 py-3 font-medium">{item.rawModel}</td>
        <td className="px-6 py-3"><Badge variant="neutral" size="sm">{item.provider}</Badge></td>
        <td className="px-6 py-3 text-right">{fmt(item.requests)}</td>
        <td className="px-6 py-3 text-right text-text-muted whitespace-nowrap">{fmtTime(item.lastUsed)}</td>
      </>
    ),
  }), [stats, sortBy, sortOrder]);

  if (!stats && !loading) return <div role="alert" className="text-error">Could not load saved gateway usage: {error || "unknown error"}. Check that the gateway is running, then reload.</div>;

  const spinner = (
    <div className="flex items-center justify-center py-12 text-text-muted">
      <span className="material-symbols-outlined text-[32px] animate-spin">progress_activity</span>
    </div>
  );

  return (
    <div className="flex min-w-0 flex-col gap-6">
      {/* Period selector (hidden when controlled by parent) */}
      {!hidePeriodSelector && (
        <div className="flex w-full items-center gap-2 sm:w-auto sm:self-end">
          <div className="grid flex-1 grid-cols-6 items-center gap-1 rounded-lg border border-border bg-bg-subtle p-1 sm:flex sm:flex-none">
            {PERIODS.map((p) => (
              <button
                key={p.value}
                onClick={() => setPeriod(p.value)}
                disabled={fetching}
                className={`rounded-md px-3 py-1 text-sm font-medium transition-colors ${period === p.value ? "bg-primary text-white shadow-sm" : "text-text-muted hover:bg-bg-hover hover:text-text"}`}
              >
                {p.label}
              </button>
            ))}
          </div>
          {fetching && (
            <span className="material-symbols-outlined text-[16px] text-text-muted animate-spin">progress_activity</span>
          )}
        </div>
      )}

      {error && <p role="alert" className="text-error">Usage reload failed: {error}. Check the gateway connection.</p>}
      {loading ? spinner : <OverviewCards stats={stats} />}

      {loading ? spinner : <RecentRequests requests={stats.recentRequests} />}

      {/* Persisted token and cost history */}
      {loading ? spinner : <UsageChart period={period} />}

      {!loading && (
        <div className="grid min-w-0 grid-cols-1 gap-2 lg:grid-cols-2">
          <ProviderBarChart byProvider={stats.byProvider} />
          <TopModelsChart byModel={stats.byModel} />
        </div>
      )}

      {/* Table with dropdown selector */}
      <div className="flex flex-col gap-3">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div className="grid grid-cols-2 items-center gap-1 rounded-lg border border-border bg-bg-subtle p-1 sm:flex">
            <button
              onClick={() => setViewMode("costs")}
              className={`px-3 py-1 rounded-md text-sm font-medium transition-colors ${viewMode === "costs" ? "bg-primary text-white shadow-sm" : "text-text-muted hover:text-text hover:bg-bg-hover"}`}
            >
              Costs
            </button>
            <button
              onClick={() => setViewMode("tokens")}
              className={`px-3 py-1 rounded-md text-sm font-medium transition-colors ${viewMode === "tokens" ? "bg-primary text-white shadow-sm" : "text-text-muted hover:text-text hover:bg-bg-hover"}`}
            >
              Tokens
            </button>
          </div>
        </div>
        {loading ? spinner : activeTableConfig && (
          <UsageTable
            title=""
            columns={activeTableConfig.columns}
            groupedData={activeTableConfig.groupedData}
            tableType="model"
            sortBy={sortBy}
            sortOrder={sortOrder}
            onToggleSort={toggleSort}
            viewMode={viewMode}
            storageKey={activeTableConfig.storageKey}
            renderSummaryCells={activeTableConfig.renderSummaryCells}
            renderDetailCells={activeTableConfig.renderDetailCells}
            emptyMessage={activeTableConfig.emptyMessage}
          />
        )}
      </div>
    </div>
  );
}
