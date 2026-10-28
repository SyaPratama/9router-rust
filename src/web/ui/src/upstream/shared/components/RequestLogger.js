"use client";

import { useState, useEffect, useCallback } from "react";
import Card from "./Card";

export default function RequestLogger({ period = "all" }) {
  const [logs, setLogs] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchLogs = useCallback(async (showLoading = true) => {
    if (showLoading) setLoading(true);
    try {
      const res = await fetch(`/api/usage/logs?period=${period}`);
      if (!res.ok) throw new Error(`Gateway returned HTTP ${res.status}`);
      const data = await res.json();
      setLogs(data.logs || []);
      setError(null);
    } catch (err) {
      setError(err.message);
    } finally {
      if (showLoading) setLoading(false);
    }
  }, [period]);

  useEffect(() => { fetchLogs(); }, [fetchLogs]);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(() => fetchLogs(false), 3000);
    return () => clearInterval(interval);
  }, [autoRefresh, fetchLogs]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold">Request Logs</h2>
        <div className="flex items-center gap-2">
          <div className="text-sm font-medium text-text-muted flex items-center gap-2">
            <span id="auto-refresh-label">Auto Refresh (3s)</span>
            <button
              type="button"
              role="switch"
              aria-checked={autoRefresh}
              aria-labelledby="auto-refresh-label"
              onClick={() => setAutoRefresh(!autoRefresh)}
              className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary ${autoRefresh ? "bg-primary" : "bg-bg-subtle border border-border"}`}
            >
              <span className={`inline-block h-3 w-3 transform rounded-full bg-white transition-transform ${autoRefresh ? "translate-x-5" : "translate-x-1"}`} />
            </button>
          </div>
        </div>
      </div>

      {error && <p role="alert" className="text-error">Could not load saved request logs: {error}. Check that the gateway is running.</p>}
      <Card className="overflow-hidden bg-black/5 dark:bg-black/20">
        <div className="p-0 overflow-x-auto max-h-[600px] overflow-y-auto font-mono text-xs">
          {loading && logs.length === 0 ? (
            <div className="p-8 text-center text-text-muted">Loading saved logs...</div>
          ) : error && logs.length === 0 ? (
            <div className="p-8 text-center text-text-muted">Logs unavailable until the gateway responds.</div>
          ) : logs.length === 0 ? (
            <div className="p-8 text-center text-text-muted">No saved requests for this period.</div>
          ) : (
            <table className="w-full text-left border-collapse whitespace-nowrap">
              <thead className="sticky top-0 bg-bg-subtle border-b border-border z-10">
                <tr>
                  <th className="px-3 py-2 border-r border-border">DateTime</th>
                  <th className="px-3 py-2 border-r border-border">Model</th>
                  <th className="px-3 py-2 border-r border-border">Provider</th>
                  <th className="px-3 py-2 border-r border-border">Input</th>
                  <th className="px-3 py-2 border-r border-border">Output</th>
                  <th className="px-3 py-2">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/50">
                {logs.map((log, i) => {
                  const parts = log.split(" | ");
                  if (parts.length < 6) return null;
                  const status = parts[5];
                  const normalizedStatus = status.toLowerCase();
                  const isFailed = normalizedStatus.includes("fail") || normalizedStatus.includes("error");
                  const isSuccess = normalizedStatus === "success" || normalizedStatus === "ok";
                  return (
                    <tr key={`${parts[0]}-${i}`} className="hover:bg-primary/5 transition-colors">
                      <td className="px-3 py-1.5 border-r border-border text-text-muted">{parts[0]}</td>
                      <td className="px-3 py-1.5 border-r border-border font-medium">{parts[1]}</td>
                      <td className="px-3 py-1.5 border-r border-border"><span className="px-1.5 py-0.5 rounded bg-bg-subtle border border-border text-[10px] uppercase font-bold">{parts[2]}</span></td>
                      <td className="px-3 py-1.5 border-r border-border text-right text-primary">{parts[3]}</td>
                      <td className="px-3 py-1.5 border-r border-border text-right text-success">{parts[4]}</td>
                      <td className={`px-3 py-1.5 font-bold ${isSuccess ? "text-success" : isFailed ? "text-error" : "text-text-muted"}`}>{status}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </Card>
      <div className="text-[10px] text-text-muted italic">Logs are read from saved gateway request records.</div>
    </div>
  );
}
