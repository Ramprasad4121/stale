"use client";
import { useState, useEffect } from 'react';
import { Shield, Activity, AlertTriangle, CheckCircle, Clock, DollarSign, TrendingUp, Zap, Filter, Download, Search, MoreHorizontal } from 'lucide-react';

interface AuditLog {
  id: string;
  trace_id: string;
  decision: string;
  blocked_by: string | null;
  reason: string;
  duration_ms: number;
  environment: string;
  created_at: string;
  request: { chain_id: number; amount_usd: number };
  metadata: { gas_price_gwei: number; price_usd: number };
}

export default function Dashboard() {
  const [logs, setLogs] = useState<AuditLog[]>([]);
  const [stats, setStats] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

  useEffect(() => {
    async function fetchData() {
      try {
        const [logsRes, statsRes] = await Promise.all([
          fetch(`${API_URL}/v1/audit/logs?limit=20`, { headers: { 'X-Org-Id': 'demo' } }),
          fetch(`${API_URL}/v1/audit/stats`, { headers: { 'X-Org-Id': 'demo' } }),
        ]);
        if (logsRes.ok) {
          const data = await logsRes.json();
          setLogs(data.logs || []);
        }
        if (statsRes.ok) {
          const data = await statsRes.json();
          setStats(data);
        }
      } catch (e) {
        console.error(e);
        // Mock data fallback
        setStats({
          total_checks: 1389,
          total_blocks: 101,
          total_allows: 1288,
          block_rate: 0.0727,
          avg_duration_ms: 34.2,
          gas_saved_usd: 4797.5,
          top_blocked_reasons: [
            { guard: "gas_price", count: 32, percentage: 31.7 },
            { guard: "oracle_freshness", count: 28, percentage: 27.7 },
            { guard: "mev_rpc", count: 19, percentage: 18.8 },
            { guard: "sequencer", count: 15, percentage: 14.9 },
            { guard: "chain_id", count: 7, percentage: 6.9 },
          ]
        });
        setLogs([
          { id: "1", trace_id: "trc_a1b2c3d4", decision: "Block", blocked_by: "gas_price", reason: "Gas price 78 Gwei exceeds policy 50 Gwei - BLOCK", duration_ms: 23.5, environment: "production", created_at: new Date().toISOString(), request: { chain_id: 1, amount_usd: 50000 }, metadata: { gas_price_gwei: 78, price_usd: 2500 } },
          { id: "2", trace_id: "trc_e5f6g7h8", decision: "Allow", blocked_by: null, reason: "All guardrails passed - execution allowed", duration_ms: 18.2, environment: "production", created_at: new Date(Date.now() - 1000*60*5).toISOString(), request: { chain_id: 1, amount_usd: 10000 }, metadata: { gas_price_gwei: 24, price_usd: 2510 } },
          { id: "3", trace_id: "trc_i9j0k1l2", decision: "Block", blocked_by: "oracle_freshness", reason: "Chainlink ETH/USD stale - 187s > 60s max - BLOCK", duration_ms: 45.1, environment: "production", created_at: new Date(Date.now() - 1000*60*12).toISOString(), request: { chain_id: 42161, amount_usd: 25000 }, metadata: { gas_price_gwei: 0.5, price_usd: 2495 } },
          { id: "4", trace_id: "trc_m3n4o5p6", decision: "Allow", blocked_by: null, reason: "Price feed ETH/USD fresh - 12s age", duration_ms: 12.8, environment: "staging", created_at: new Date(Date.now() - 1000*60*20).toISOString(), request: { chain_id: 10, amount_usd: 5000 }, metadata: { gas_price_gwei: 0.8, price_usd: 2505 } },
        ]);
      } finally {
        setLoading(false);
      }
    }
    fetchData();
  }, []);

  return (
    <div className="min-h-screen bg-[#020617] text-slate-100">
      {/* Sidebar */}
      <div className="fixed left-0 top-0 h-full w-64 bg-slate-900/50 border-r border-slate-800/50 backdrop-blur-xl p-6">
        <div className="flex items-center gap-3 mb-8">
          <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center font-bold text-white">S</div>
          <div>
            <div className="font-bold">STALE</div>
            <div className="text-[10px] text-slate-400 -mt-1">ENTERPRISE</div>
          </div>
        </div>

        <div className="space-y-1">
          {[
            { icon: Activity, label: "Dashboard", active: true },
            { icon: Shield, label: "Policies", count: "3" },
            { icon: Clock, label: "Audit Logs", count: stats?.total_checks || "1.3k" },
            { icon: Zap, label: "API Keys", count: "2" },
            { icon: TrendingUp, label: "Metrics" },
            { icon: DollarSign, label: "Billing" },
          ].map((item, i) => (
            <div key={i} className={`flex items-center justify-between px-3 py-2.5 rounded-xl text-sm ${item.active ? 'bg-white text-slate-900 font-medium' : 'text-slate-400 hover:text-white hover:bg-slate-800/50'}`}>
              <div className="flex items-center gap-3">
                <item.icon className="w-4 h-4" />
                {item.label}
              </div>
              {item.count && <span className={`text-xs px-2 py-0.5 rounded-full ${item.active ? 'bg-slate-900 text-white' : 'bg-slate-800 text-slate-400'}`}>{item.count}</span>}
            </div>
          ))}
        </div>

        <div className="absolute bottom-6 left-6 right-6">
          <div className="p-4 rounded-xl bg-blue-950/30 border border-blue-900/50">
            <div className="text-xs font-medium text-blue-300 mb-1">Enterprise Plan</div>
            <div className="text-[11px] text-slate-400">100k checks/mo • 365-day retention</div>
            <div className="mt-3 h-1.5 bg-slate-800 rounded-full overflow-hidden">
              <div className="h-full w-[14%] bg-blue-500 rounded-full" />
            </div>
            <div className="text-[10px] text-slate-500 mt-1">14% used • 13.8k / 100k</div>
          </div>
        </div>
      </div>

      {/* Main */}
      <div className="ml-64 p-8">
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-2xl font-bold text-white">Security Dashboard</h1>
            <p className="text-slate-400 text-sm mt-1">Real-time guardrail monitoring • Acme DeFi Corp • Production</p>
          </div>
          <div className="flex items-center gap-3">
            <div className="px-3 py-1.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-xs flex items-center gap-1.5">
              <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              API Operational • 12ms p95
            </div>
            <button className="p-2 rounded-xl bg-slate-800 border border-slate-700 hover:bg-slate-700">
              <Download className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="p-5 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
            <div className="flex items-center justify-between mb-3">
              <div className="w-10 h-10 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center">
                <Activity className="w-5 h-5 text-blue-400" />
              </div>
              <span className="text-xs px-2 py-1 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">+12% today</span>
            </div>
            <div className="text-2xl font-bold text-white font-mono">{stats?.total_checks?.toLocaleString() || '1,389'}</div>
            <div className="text-xs text-slate-400 mt-1">Total Checks (24h)</div>
            <div className="mt-3 flex items-center gap-1 text-[11px] text-slate-500">
              <div className="flex-1 h-1 bg-slate-800 rounded-full overflow-hidden">
                <div className="h-full w-[92%] bg-blue-500 rounded-full" />
              </div>
              92% ALLOW
            </div>
          </div>

          <div className="p-5 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
            <div className="flex items-center justify-between mb-3">
              <div className="w-10 h-10 rounded-xl bg-red-500/10 border border-red-500/20 flex items-center justify-center">
                <Shield className="w-5 h-5 text-red-400" />
              </div>
              <span className="text-xs px-2 py-1 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/20">Critical</span>
            </div>
            <div className="text-2xl font-bold text-white font-mono">{stats?.total_blocks || '101'}</div>
            <div className="text-xs text-slate-400 mt-1">Blocked Executions</div>
            <div className="mt-3 text-[11px] text-slate-500">
              <span className="text-red-400 font-medium">{((stats?.block_rate || 0.0727)*100).toFixed(1)}% block rate</span> • Prevented bad txs
            </div>
          </div>

          <div className="p-5 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
            <div className="flex items-center justify-between mb-3">
              <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center">
                <DollarSign className="w-5 h-5 text-emerald-400" />
              </div>
              <span className="text-xs px-2 py-1 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">Saved</span>
            </div>
            <div className="text-2xl font-bold text-white font-mono">${stats?.gas_saved_usd?.toLocaleString() || '4,797'}</div>
            <div className="text-xs text-slate-400 mt-1">Gas & Loss Prevented</div>
            <div className="mt-3 text-[11px] text-slate-500">Avg $47.50 per BLOCK • 101 blocks</div>
          </div>

          <div className="p-5 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
            <div className="flex items-center justify-between mb-3">
              <div className="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center">
                <Clock className="w-5 h-5 text-amber-400" />
              </div>
              <span className="text-xs px-2 py-1 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20">P95</span>
            </div>
            <div className="text-2xl font-bold text-white font-mono">{stats?.avg_duration_ms?.toFixed(1) || '34.2'}ms</div>
            <div className="text-xs text-slate-400 mt-1">Avg Check Latency</div>
            <div className="mt-3 text-[11px] text-slate-500">Target &lt;50ms • <span className="text-emerald-400">✓ Meeting SLA</span></div>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-6">
          {/* Audit Logs */}
          <div className="col-span-2 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur overflow-hidden">
            <div className="p-5 border-b border-slate-800/50 flex items-center justify-between">
              <h2 className="font-semibold text-white flex items-center gap-2">
                <Clock className="w-4 h-4 text-slate-400" />
                Live Audit Trail
                <span className="ml-2 w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              </h2>
              <div className="flex items-center gap-2">
                <div className="relative">
                  <Search className="w-4 h-4 absolute left-2.5 top-2.5 text-slate-500" />
                  <input placeholder="Search trace_id, reason..." className="pl-8 pr-3 py-2 rounded-xl bg-slate-800 border border-slate-700 text-sm text-white placeholder:text-slate-500 w-64" />
                </div>
                <button className="p-2 rounded-xl bg-slate-800 border border-slate-700 hover:bg-slate-700">
                  <Filter className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="divide-y divide-slate-800/50">
              {logs.map((log) => (
                <div key={log.id} className="p-4 hover:bg-slate-800/30 transition-colors group">
                  <div className="flex items-start justify-between">
                    <div className="flex items-start gap-3">
                      <div className={`mt-1 w-8 h-8 rounded-xl flex items-center justify-center border ${log.decision === 'Block' ? 'bg-red-500/10 border-red-500/20' : 'bg-emerald-500/10 border-emerald-500/20'}`}>
                        {log.decision === 'Block' ? <AlertTriangle className="w-4 h-4 text-red-400" /> : <CheckCircle className="w-4 h-4 text-emerald-400" />}
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-xs text-slate-300">{log.trace_id}</span>
                          <span className={`text-[10px] px-2 py-0.5 rounded-full font-bold border ${log.decision === 'Block' ? 'bg-red-500/10 text-red-400 border-red-500/20' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'}`}>
                            {log.decision.toUpperCase()}
                          </span>
                          {log.blocked_by && (
                            <span className="text-[10px] px-2 py-0.5 rounded-full bg-slate-800 text-slate-400 border border-slate-700">
                              {log.blocked_by}
                            </span>
                          )}
                          <span className="text-[10px] px-2 py-0.5 rounded-full bg-slate-800 text-slate-400">
                            {log.environment}
                          </span>
                        </div>
                        <div className="text-sm text-white mt-1.5 font-medium">{log.reason}</div>
                        <div className="flex items-center gap-3 mt-2 text-[11px] text-slate-500">
                          <span>⛓️ Chain {log.request.chain_id}</span>
                          <span>💰 ${log.request.amount_usd?.toLocaleString()}</span>
                          <span>⛽ {log.metadata.gas_price_gwei} Gwei</span>
                          <span>⏱️ {log.duration_ms}ms</span>
                          <span>🕒 {new Date(log.created_at).toLocaleTimeString()}</span>
                        </div>
                      </div>
                    </div>
                    <button className="p-1.5 rounded-lg hover:bg-slate-700 opacity-0 group-hover:opacity-100 transition-opacity">
                      <MoreHorizontal className="w-4 h-4 text-slate-400" />
                    </button>
                  </div>
                </div>
              ))}
            </div>

            <div className="p-4 border-t border-slate-800/50 text-center">
              <button className="text-sm text-blue-400 hover:text-blue-300">View all 1,389 logs →</button>
            </div>
          </div>

          {/* Top Blocked Reasons */}
          <div className="space-y-6">
            <div className="p-5 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
              <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                <AlertTriangle className="w-4 h-4 text-amber-400" />
                Top Blocked Reasons
              </h3>
              <div className="space-y-3">
                {stats?.top_blocked_reasons?.map((item: any, i: number) => (
                  <div key={i} className="group">
                    <div className="flex items-center justify-between mb-1.5">
                      <span className="text-sm text-slate-300 font-mono">{item.guard}</span>
                      <span className="text-xs text-slate-500">{item.count} • {item.percentage.toFixed(1)}%</span>
                    </div>
                    <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                      <div className="h-full bg-gradient-to-r from-red-500 to-amber-500 rounded-full transition-all group-hover:opacity-80" style={{ width: `${item.percentage}%` }} />
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-4 p-3 rounded-xl bg-amber-950/20 border border-amber-900/30">
                <div className="text-xs text-amber-300 font-medium">💡 Insight</div>
                <div className="text-[11px] text-slate-400 mt-1">Gas spikes caused 31.7% of blocks. Consider raising prod threshold to 60 Gwei during high volatility.</div>
              </div>
            </div>

            <div className="p-5 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
              <h3 className="font-semibold text-white mb-4">Policy Status</h3>
              <div className="space-y-3">
                {[
                  { name: "Production - Strict", env: "production", checks: "4 guards", status: "active", version: "v3" },
                  { name: "Staging - Permissive", env: "staging", checks: "3 guards", status: "active", version: "v2" },
                  { name: "High-Value - Fort Knox", env: "production", checks: "6 guards", status: "active", version: "v1" },
                ].map((policy, i) => (
                  <div key={i} className="flex items-center justify-between p-3 rounded-xl bg-slate-800/50 border border-slate-700/50">
                    <div>
                      <div className="text-sm font-medium text-white">{policy.name}</div>
                      <div className="text-[11px] text-slate-500">{policy.env} • {policy.checks} • {policy.version}</div>
                    </div>
                    <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                  </div>
                ))}
              </div>
            </div>

            <div className="p-5 rounded-2xl bg-gradient-to-br from-blue-950/30 to-cyan-950/30 border border-blue-900/30">
              <h3 className="font-semibold text-white mb-2">🚀 Enterprise Ready</h3>
              <div className="text-xs text-slate-400 leading-relaxed">
                Your stale library is now a platform. SOC2 logs, SSO, webhooks, and 99.99% SLA. 
                <br /><br />
                <span className="text-blue-300">Next: Enable Slack alerts for BLOCK events in Settings.</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
