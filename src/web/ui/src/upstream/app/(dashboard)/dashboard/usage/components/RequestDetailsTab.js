"use client";

import { useState, useEffect, useCallback } from "react";
import Card from "@/shared/components/Card";
import Button from "@/shared/components/Button";
import Drawer from "@/shared/components/Drawer";
import Pagination from "@/shared/components/Pagination";
import { cn } from "@/shared/utils/cn";
import { AI_PROVIDERS, getProviderByAlias } from "@/shared/constants/providers";


function getProviderName(providerId) {
  if (!providerId) return "Unknown";
  const provider = getProviderByAlias(providerId) || AI_PROVIDERS[providerId];
  return provider?.name || providerId;
}
const formatTokens = (value) => value == null ? "Unavailable" : value.toLocaleString();
const formatCost = (value) => value == null ? "Unavailable" : `$${value.toFixed(4)}`;

export default function RequestDetailsTab({ period = "all" }) {
  const [details, setDetails] = useState([]);
  const [pagination, setPagination] = useState({
    page: 1,
    pageSize: 20,
    totalItems: 0,
    totalPages: 0
  });
  const [loading, setLoading] = useState(false);
  const [selectedDetail, setSelectedDetail] = useState(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [providers, setProviders] = useState([]);
  const [error, setError] = useState(null);
  const [filters, setFilters] = useState({
    provider: "",
    startDate: "",
    endDate: ""
  });

  const fetchProviders = useCallback(async () => {
    try {
      const res = await fetch(`/api/usage/history?period=${period}`);
      if (!res.ok) throw new Error(`Gateway returned HTTP ${res.status}`);
      const data = await res.json();
      const ids = [...new Set((data.breakdown || []).map((row) => row.provider))];
      setProviders(ids.map((id) => ({ id, name: getProviderName(id) })));
    } catch {
      setProviders([]);
    }
  }, [period]);

  const fetchDetails = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams({
        page: pagination.page.toString(),
        pageSize: pagination.pageSize.toString(),
        period,
      });
      if (filters.provider) params.set("provider", filters.provider);
      if (filters.startDate) params.set("startDate", new Date(filters.startDate).toISOString());
      if (filters.endDate) params.set("endDate", new Date(filters.endDate).toISOString());

      const res = await fetch(`/api/usage/request-details?${params}`);
      if (!res.ok) throw new Error(`Gateway returned HTTP ${res.status}`);
      const data = await res.json();
      setDetails(data.details || []);
      setPagination((prev) => ({ ...prev, ...data.pagination }));
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }, [pagination.page, pagination.pageSize, filters, period]);

  useEffect(() => {
    fetchProviders();
  }, [fetchProviders]);

  useEffect(() => {
    fetchDetails();
  }, [fetchDetails]);

  useEffect(() => {
    setPagination((prev) => ({ ...prev, page: 1 }));
  }, [period]);

  const updateFilter = (field, value) => {
    setFilters((prev) => ({ ...prev, [field]: value }));
    setPagination((prev) => ({ ...prev, page: 1 }));
  };

  const handleViewDetail = (detail) => {
    setSelectedDetail(detail);
    setIsDrawerOpen(true);
  };

  const handlePageChange = (newPage) => setPagination((prev) => ({ ...prev, page: newPage }));
  const handlePageSizeChange = (newPageSize) => setPagination((prev) => ({ ...prev, pageSize: newPageSize, page: 1 }));
  const handleClearFilters = () => {
    setFilters({ provider: "", startDate: "", endDate: "" });
    setPagination((prev) => ({ ...prev, page: 1 }));
  };

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <Card padding="md">
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <div className="flex min-w-0 flex-col gap-2">
            <label htmlFor="provider-filter" className="text-sm font-medium text-text-main">Provider</label>
            <select
              id="provider-filter"
              value={filters.provider}
              onChange={(e) => updateFilter("provider", e.target.value)}
              className={cn(
                "h-9 px-3 rounded-lg border border-black/10 dark:border-white/10 bg-surface",
                "text-sm text-text-main focus:outline-none focus:ring-2 focus:ring-primary/20",
                "w-full min-w-0 cursor-pointer"
              )}
              style={{ colorScheme: 'auto' }}
            >
              <option value="">All Providers</option>
              {providers.map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.name}
                </option>
              ))}
            </select>
          </div>
          
          <div className="flex min-w-0 flex-col gap-2">
            <label htmlFor="start-date-filter" className="text-sm font-medium text-text-main">Start Date</label>
            <input
              id="start-date-filter"
              type="datetime-local"
              value={filters.startDate}
              onChange={(e) => updateFilter("startDate", e.target.value)}
              className={cn(
                "h-9 px-3 rounded-lg border border-black/10 dark:border-white/10 bg-surface",
                "w-full min-w-0 text-sm text-text-main focus:outline-none focus:ring-2 focus:ring-primary/20"
              )}
            />
          </div>

          <div className="flex min-w-0 flex-col gap-2">
            <label htmlFor="end-date-filter" className="text-sm font-medium text-text-main">End Date</label>
            <input
              id="end-date-filter"
              type="datetime-local"
              value={filters.endDate}
              onChange={(e) => updateFilter("endDate", e.target.value)}
              className={cn(
                "h-9 px-3 rounded-lg border border-black/10 dark:border-white/10 bg-surface",
                "w-full min-w-0 text-sm text-text-main focus:outline-none focus:ring-2 focus:ring-primary/20"
              )}
            />
          </div>
          
          <div className="flex min-w-0 flex-col gap-2 sm:col-span-2 lg:col-span-1">
            <span className="hidden text-sm font-medium text-text-main opacity-0 lg:block" aria-hidden="true">Clear</span>
            <Button 
              variant="ghost" 
              onClick={handleClearFilters}
              disabled={!filters.provider && !filters.startDate && !filters.endDate}
              className="w-full"
            >
              Clear Filters
            </Button>
          </div>
        </div>
      </Card>

      {error && <p role="alert" className="text-error">Could not load saved request details: {error}. Check the gateway connection and filter values.</p>}
      <Card padding="none">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[900px]">
            <thead>
              <tr className="border-b border-black/5 dark:border-white/5">
                <th className="text-left p-4 text-sm font-semibold text-text-main">Timestamp</th>
                <th className="text-left p-4 text-sm font-semibold text-text-main">Model</th>
                <th className="text-left p-4 text-sm font-semibold text-text-main">Provider</th>
                <th className="text-right p-4 text-sm font-semibold text-text-main">Input</th>
                <th className="text-right p-4 text-sm font-semibold text-text-main">Output</th>
                <th className="text-left p-4 text-sm font-semibold text-text-main">Status</th>
                <th className="text-right p-4 text-sm font-semibold text-text-main">Reported Cost</th>
                <th className="text-center p-4 text-sm font-semibold text-text-main">Details</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                <tr><td colSpan="8" className="p-8 text-center text-text-muted">Loading saved request records...</td></tr>
              ) : details.length === 0 ? (
                <tr><td colSpan="8" className="p-8 text-center text-text-muted">No saved requests match this period and filter.</td></tr>
              ) : details.map((detail, index) => (
                <tr key={`${detail.id}-${index}`} className="border-b border-black/5 dark:border-white/5 last:border-b-0 hover:bg-black/[0.02] dark:hover:bg-white/[0.02]">
                  <td className="whitespace-nowrap p-4 text-sm text-text-main">{new Date(detail.timestamp).toLocaleString()}</td>
                  <td className="max-w-[260px] truncate p-4 font-mono text-sm text-text-main">{detail.model || "Unknown"}</td>
                  <td className="max-w-[180px] truncate p-4 text-sm text-text-main">{getProviderName(detail.provider)}</td>
                  <td className="p-4 text-sm text-text-main text-right font-mono">{formatTokens(detail.promptTokens)}</td>
                  <td className="p-4 text-sm text-text-main text-right font-mono">{formatTokens(detail.completionTokens)}</td>
                  <td className="p-4 text-sm text-text-main">{detail.status || "Unavailable"}</td>
                  <td className="p-4 text-sm text-text-main text-right font-mono">{formatCost(detail.cost)}</td>
                  <td className="p-4 text-center"><Button variant="outline" size="sm" onClick={() => handleViewDetail(detail)}>Details</Button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {!loading && details.length > 0 && (
          <div className="border-t border-black/5 dark:border-white/5">
            <Pagination currentPage={pagination.page} pageSize={pagination.pageSize} totalItems={pagination.totalItems} onPageChange={handlePageChange} onPageSizeChange={handlePageSizeChange} />
          </div>
        )}
      </Card>


      <Drawer
        isOpen={isDrawerOpen}
        onClose={() => setIsDrawerOpen(false)}
        title="Request Details"
        width="lg"
      >
        {selectedDetail && (
          <dl className="grid min-w-0 grid-cols-1 gap-4 text-sm sm:grid-cols-2">
            <div><dt className="text-text-muted">ID</dt><dd className="break-all font-mono text-text-main">{selectedDetail.id ?? "Unavailable"}</dd></div>
            <div><dt className="text-text-muted">Timestamp</dt><dd className="text-text-main">{new Date(selectedDetail.timestamp).toLocaleString()}</dd></div>
            <div><dt className="text-text-muted">Provider</dt><dd className="font-medium text-text-main">{getProviderName(selectedDetail.provider)}</dd></div>
            <div><dt className="text-text-muted">Model</dt><dd className="break-all font-mono text-text-main">{selectedDetail.model || "Unavailable"}</dd></div>
            <div><dt className="text-text-muted">Status</dt><dd className="text-text-main">{selectedDetail.status || "Unavailable"}</dd></div>
            <div><dt className="text-text-muted">Endpoint</dt><dd className="break-all font-mono text-text-main">{selectedDetail.endpoint || "Unavailable"}</dd></div>
            <div><dt className="text-text-muted">Connection</dt><dd className="break-all font-mono text-text-main">{selectedDetail.connectionId || "Unavailable"}</dd></div>
            <div><dt className="text-text-muted">Input Tokens</dt><dd className="font-mono text-text-main">{formatTokens(selectedDetail.promptTokens)}</dd></div>
            <div><dt className="text-text-muted">Output Tokens</dt><dd className="font-mono text-text-main">{formatTokens(selectedDetail.completionTokens)}</dd></div>
            <div><dt className="text-text-muted">Reported Cost</dt><dd className="font-mono text-text-main">{formatCost(selectedDetail.cost)}</dd></div>
          </dl>
        )}
      </Drawer>
    </div>
  );
}
