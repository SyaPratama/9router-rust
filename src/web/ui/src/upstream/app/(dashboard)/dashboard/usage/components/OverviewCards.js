"use client";

import PropTypes from "prop-types";
import Card from "@/shared/components/Card";

const fmt = (n) => n == null ? "Unavailable" : new Intl.NumberFormat().format(n);
const fmtCost = (n) => n == null ? "Unavailable" : `$${n.toFixed(4)}`;

export default function OverviewCards({ stats }) {
  return (
    <div className="grid min-w-0 grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4 sm:gap-4">
      <Card className="flex min-w-0 flex-col items-center text-center gap-1 px-3 py-3 sm:px-4">
        <span className="text-text-muted text-xs uppercase font-semibold sm:text-sm">Total Requests</span>
        <span className="w-full truncate text-lg font-bold xl:text-xl" title={fmt(stats.totalRequests)}>{fmt(stats.totalRequests)}</span>
      </Card>
      <Card className="flex min-w-0 flex-col items-center text-center gap-1 px-3 py-3 sm:px-4">
        <span className="text-text-muted text-xs uppercase font-semibold sm:text-sm">Total Input Tokens</span>
        <span className="w-full truncate text-lg font-bold text-primary xl:text-xl" title={fmt(stats.totalPromptTokens)}>{fmt(stats.totalPromptTokens)}</span>
      </Card>
      <Card className="flex min-w-0 flex-col items-center text-center gap-1 px-3 py-3 sm:px-4">
        <span className="text-text-muted text-xs uppercase font-semibold sm:text-sm">Output Tokens</span>
        <span className="w-full truncate text-lg font-bold text-success xl:text-xl" title={fmt(stats.totalCompletionTokens)}>{fmt(stats.totalCompletionTokens)}</span>
      </Card>
      <Card className="flex min-w-0 flex-col items-center text-center gap-1 px-3 py-3 sm:px-4">
        <span className="text-text-muted text-xs uppercase font-semibold sm:text-sm">Reported Cost</span>
        <span className="w-full truncate text-lg font-bold text-warning xl:text-xl" title={fmtCost(stats.totalCost)}>{fmtCost(stats.totalCost)}</span>
      </Card>
    </div>
  );
}

OverviewCards.propTypes = {
  stats: PropTypes.object.isRequired,
};
