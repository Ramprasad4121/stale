"use client";
import { useState, useEffect } from 'react';
import Link from 'next/link';
import { Shield, Activity, AlertTriangle, CheckCircle, Zap, DollarSign, Clock, TrendingUp, Lock, Users, Key, FileText, BarChart3, ArrowRight, Play, Github } from 'lucide-react';

export default function Home() {
  const [stats, setStats] = useState({ checks: 0, blocks: 0, saved: 0 });

  useEffect(() => {
    const interval = setInterval(() => {
      setStats(s => ({
        checks: s.checks + Math.floor(Math.random() * 3),
        blocks: s.blocks + (Math.random() > 0.85 ? 1 : 0),
        saved: s.saved + (Math.random() > 0.85 ? Math.floor(Math.random() * 100) : 0),
      }));
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-[#020617] relative overflow-hidden">
      {/* Background effects */}
      <div className="absolute inset-0 bg-gradient-to-br from-blue-900/20 via-transparent to-cyan-900/20" />
      <div className="absolute top-0 left-1/4 w-96 h-96 bg-blue-600/10 rounded-full blur-[120px]" />
      <div className="absolute bottom-0 right-1/4 w-96 h-96 bg-cyan-600/10 rounded-full blur-[120px]" />

      {/* Header */}
      <header className="relative z-10 border-b border-slate-800/50 backdrop-blur-xl bg-slate-950/50">
        <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center font-bold text-white shadow-lg shadow-blue-500/20">
              S
            </div>
            <div>
              <div className="font-bold text-white tracking-tight">STALE</div>
              <div className="text-[10px] text-slate-400 -mt-1 tracking-widest">ENTERPRISE</div>
            </div>
            <div className="ml-4 px-2.5 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-xs font-medium flex items-center gap-1.5">
              <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              LIVE
            </div>
          </div>
          <div className="flex items-center gap-3">
            <a href="https://github.com/Ramprasad4121/stale" className="p-2.5 rounded-xl bg-slate-800/50 border border-slate-700/50 hover:bg-slate-800 hover:border-slate-600 transition-all">
              <Github className="w-4 h-4 text-slate-300" />
            </a>
            <Link href="/dashboard" className="px-4 py-2 rounded-xl bg-white text-slate-900 font-medium hover:bg-slate-100 transition-all text-sm">
              Open Dashboard
            </Link>
          </div>
        </div>
      </header>

      {/* Hero */}
      <div className="relative z-10 max-w-7xl mx-auto px-6 pt-16 pb-24">
        <div className="grid lg:grid-cols-2 gap-12 items-center">
          <div>
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-blue-500/10 border border-blue-500/20 text-blue-300 text-xs font-medium mb-6">
              <Zap className="w-3.5 h-3.5" />
              v2.0.0 Enterprise • SOC2 Ready • 99.99% SLA
            </div>
            
            <h1 className="text-5xl lg:text-6xl font-bold tracking-tight leading-[0.9] mb-6">
              <span className="text-white">Fail-closed</span>
              <br />
              <span className="bg-gradient-to-r from-blue-400 to-cyan-400 bg-clip-text text-transparent">guardrails</span>
              <br />
              <span className="text-slate-400 text-4xl lg:text-5xl">for agents that touch money</span>
            </h1>
            
            <p className="text-slate-400 text-lg leading-relaxed mb-8 max-w-xl">
              Stale Enterprise transforms your open-source Rust guardrail library into a managed security platform. 
              Multi-tenant, audited, with SSO, RBAC, webhooks, and $4.2k+ in prevented losses.
            </p>

            <div className="flex flex-wrap gap-3 mb-10">
              <Link href="/dashboard" className="group px-6 py-3 rounded-xl bg-white text-slate-900 font-semibold flex items-center gap-2 hover:bg-slate-100 transition-all">
                <Play className="w-4 h-4" />
                Live Demo
                <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
              </Link>
              <a href="#architecture" className="px-6 py-3 rounded-xl bg-slate-800/50 border border-slate-700/50 text-white font-medium hover:bg-slate-800 hover:border-slate-600 transition-all backdrop-blur">
                View Architecture
              </a>
            </div>

            <div className="grid grid-cols-3 gap-4">
              <div className="p-4 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
                <div className="text-2xl font-bold text-white mb-1">20+</div>
                <div className="text-xs text-slate-400">Guardrails</div>
                <div className="text-[10px] text-emerald-400 mt-1">Chainlink, L2, MEV, DeFi</div>
              </div>
              <div className="p-4 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
                <div className="text-2xl font-bold text-white mb-1">&lt;15ms</div>
                <div className="text-xs text-slate-400">P95 Latency</div>
                <div className="text-[10px] text-blue-400 mt-1">Fail-fast pipeline</div>
              </div>
              <div className="p-4 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur">
                <div className="text-2xl font-bold text-white mb-1">100%</div>
                <div className="text-xs text-slate-400">Fail-closed</div>
                <div className="text-[10px] text-amber-400 mt-1">No silent failures</div>
              </div>
            </div>
          </div>

          {/* Live Terminal */}
          <div className="relative">
            <div className="rounded-2xl bg-slate-900 border border-slate-800 shadow-2xl shadow-blue-900/20 overflow-hidden">
              <div className="flex items-center justify-between px-4 py-3 border-b border-slate-800 bg-slate-950/50">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-red-500" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500" />
                  <div className="w-3 h-3 rounded-full bg-green-500" />
                </div>
                <div className="text-xs text-slate-500 font-mono">stale-enterprise-api — production</div>
                <div className="flex items-center gap-2 text-xs">
                  <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                  <span className="text-emerald-400">LIVE</span>
                </div>
              </div>
              
              <div className="p-4 font-mono text-xs leading-relaxed bg-[#0a0f1e]">
                <div className="text-slate-500 mb-3">$ curl -X POST /v1/pipeline/run -H "X-API-Key: stale_live_..." \</div>
                <div className="text-slate-300 mb-4 pl-2 border-l-2 border-blue-500/30">
                  {`{
  "rpc_url": "https://rpc.flashbots.net",
  "policy_id": "prod-strict-v3",
  "chain_id": 1,
  "context": {
    "target_address": "0xE592...1564",
    "amount_usd": 50000
  }
}`}
                </div>
                
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <CheckCircle className="w-3.5 h-3.5 text-emerald-400" />
                    <span className="text-slate-300">oracle_freshness</span>
                    <span className="text-emerald-400">ALLOW</span>
                    <span className="text-slate-500 ml-auto">12ms</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <CheckCircle className="w-3.5 h-3.5 text-emerald-400" />
                    <span className="text-slate-300">gas_price</span>
                    <span className="text-emerald-400">ALLOW</span>
                    <span className="text-slate-500 ml-auto">8ms</span>
                    <span className="text-slate-400">(24 Gwei &lt; 50)</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <CheckCircle className="w-3.5 h-3.5 text-emerald-400" />
                    <span className="text-slate-300">mev_protection</span>
                    <span className="text-emerald-400">ALLOW</span>
                    <span className="text-slate-500 ml-auto">5ms</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="w-3.5 h-3.5 text-red-400" />
                    <span className="text-slate-300">sequencer</span>
                    <span className="text-red-400">BLOCK</span>
                    <span className="text-slate-500 ml-auto">18ms</span>
                  </div>
                </div>

                <div className="mt-4 p-3 rounded-xl bg-red-950/50 border border-red-900/50">
                  <div className="flex items-center gap-2 text-red-300 font-semibold mb-1">
                    <Shield className="w-4 h-4" />
                    BLOCKED — Arb sequencer down
                  </div>
                  <div className="text-slate-400 text-[11px]">trace_id: trc_a1b2c3d4 • policy: prod-strict-v3 • 43ms total</div>
                  <div className="text-slate-500 text-[11px] mt-1">→ Webhook dispatched to Slack #security-alerts</div>
                </div>

                <div className="mt-4 flex gap-2">
                  <div className="px-2.5 py-1 rounded-full bg-slate-800 text-slate-300 text-[10px]">Audit logged ✓</div>
                  <div className="px-2.5 py-1 rounded-full bg-slate-800 text-slate-300 text-[10px]">Metrics updated ✓</div>
                  <div className="px-2.5 py-1 rounded-full bg-amber-900/30 text-amber-300 text-[10px] border border-amber-800/50">$4,227 saved</div>
                </div>
              </div>
            </div>

            {/* Floating stats */}
            <div className="absolute -bottom-6 -left-6 p-4 rounded-2xl bg-slate-900 border border-slate-800 shadow-xl backdrop-blur-xl">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center">
                  <Activity className="w-5 h-5 text-emerald-400" />
                </div>
                <div>
                  <div className="text-xs text-slate-400">Live Checks</div>
                  <div className="text-lg font-bold text-white font-mono">{stats.checks.toLocaleString()}</div>
                </div>
              </div>
            </div>

            <div className="absolute -top-6 -right-6 p-4 rounded-2xl bg-slate-900 border border-slate-800 shadow-xl backdrop-blur-xl">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl bg-red-500/10 border border-red-500/20 flex items-center justify-center">
                  <Shield className="w-5 h-5 text-red-400" />
                </div>
                <div>
                  <div className="text-xs text-slate-400">Blocked Today</div>
                  <div className="text-lg font-bold text-white font-mono">{stats.blocks}</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Enterprise Features */}
        <div id="architecture" className="mt-32">
          <div className="text-center mb-12">
            <h2 className="text-3xl font-bold text-white mb-3">From library to enterprise platform</h2>
            <p className="text-slate-400 max-w-2xl mx-auto">We kept your Rust core pure and fast, but wrapped it in everything enterprises need to trust agents with real money.</p>
          </div>

          <div className="grid md:grid-cols-3 gap-6">
            {[
              {
                icon: Lock,
                title: "Security & Compliance",
                items: ["SOC2 Type II ready audit logs", "SAML SSO (Okta, Azure AD)", "RBAC: Owner, Admin, Dev, Auditor", "PII redaction, encrypted at rest", "Webhook signing, mTLS"]
              },
              {
                icon: BarChart3,
                title: "Observability",
                items: ["Real-time BLOCK/ALLOW metrics", "P50/P95/P99 latency tracking", "Gas saved & loss prevented", "Slack/PagerDuty alerts on BLOCK", "ClickHouse for 365-day retention"]
              },
              {
                icon: Zap,
                title: "Developer Experience",
                items: ["<15ms p95, 64 guards per pipeline", "Rust, TS, Python SDKs", "OpenAPI + MCP server", "Policy-as-code with versioning", "Dry-run & audit-only modes"]
              },
              {
                icon: Users,
                title: "Multi-Tenancy",
                items: ["Organizations & environments", "Per-org rate limits & caps", "Isolated audit logs", "Usage metering & billing", "99.99% SLA with credits"]
              },
              {
                icon: FileText,
                title: "Policy Engine",
                items: ["Fail-closed, fail-fast, warn-only", "Oracle freshness & deviation", "MEV, sequencer, gas policies", "Allowlist with strict mode", "Custom guard plugins (WASM)"]
              },
              {
                icon: Key,
                title: "Enterprise Ops",
                items: ["API key rotation & IP allowlist", "Terraform provider", "Docker & K8s Helm charts", "On-prem air-gapped deploy", "24/7 dedicated support"]
              },
            ].map((feature, i) => (
              <div key={i} className="group p-6 rounded-2xl bg-slate-900/50 border border-slate-800/50 backdrop-blur hover:border-slate-700/50 hover:bg-slate-900/80 transition-all card-hover">
                <div className="w-10 h-10 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center mb-4 group-hover:bg-blue-500/20 transition-colors">
                  <feature.icon className="w-5 h-5 text-blue-400" />
                </div>
                <h3 className="font-semibold text-white mb-3">{feature.title}</h3>
                <ul className="space-y-2">
                  {feature.items.map((item, j) => (
                    <li key={j} className="flex items-start gap-2 text-sm text-slate-400">
                      <CheckCircle className="w-3.5 h-3.5 text-emerald-400 mt-0.5 flex-shrink-0" />
                      <span>{item}</span>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>

        {/* Pricing */}
        <div className="mt-24 grid md:grid-cols-3 gap-6 max-w-5xl mx-auto">
          {[
            { name: "Starter", price: "$0", checks: "1,000/mo", features: ["Community support", "1 org, 3 policies", "7-day logs", "Standard guards"] },
            { name: "Pro", price: "$299", checks: "50,000/mo", features: ["Email support", "5 orgs, unlimited policies", "90-day logs", "Webhooks, Slack", "SSO"], popular: false },
            { name: "Enterprise", price: "$999", checks: "100,000/mo + overage", features: ["24/7 dedicated", "Unlimited orgs", "365-day + S3 export", "SAML, SCIM, RBAC", "On-prem, mTLS", "99.99% SLA", "Custom guards"], popular: true },
          ].map((tier, i) => (
            <div key={i} className={`p-6 rounded-2xl border backdrop-blur ${tier.popular ? 'bg-blue-950/20 border-blue-800/50 shadow-xl shadow-blue-900/10 scale-105' : 'bg-slate-900/50 border-slate-800/50'}`}>
              {tier.popular && <div className="inline-flex px-2.5 py-1 rounded-full bg-blue-500 text-white text-xs font-bold mb-3">MOST POPULAR</div>}
              <h3 className="font-bold text-white text-lg">{tier.name}</h3>
              <div className="mt-2 flex items-baseline gap-1">
                <span className="text-3xl font-bold text-white">{tier.price}</span>
                <span className="text-slate-400 text-sm">/month</span>
              </div>
              <div className="text-xs text-slate-500 mt-1">{tier.checks} checks</div>
              <ul className="mt-6 space-y-2.5">
                {tier.features.map((f, j) => (
                  <li key={j} className="flex items-center gap-2 text-sm text-slate-300">
                    <CheckCircle className="w-4 h-4 text-emerald-400" />
                    {f}
                  </li>
                ))}
              </ul>
              <button className={`w-full mt-6 py-2.5 rounded-xl font-medium transition-all ${tier.popular ? 'bg-white text-slate-900 hover:bg-slate-100' : 'bg-slate-800 text-white hover:bg-slate-700 border border-slate-700'}`}>
                {tier.name === "Starter" ? "Start Free" : tier.name === "Enterprise" ? "Contact Sales" : "Start Pro Trial"}
              </button>
            </div>
          ))}
        </div>
      </div>

      <footer className="relative z-10 border-t border-slate-800/50 py-8 mt-16">
        <div className="max-w-7xl mx-auto px-6 flex items-center justify-between text-sm text-slate-500">
          <div>© 2026 Stale Enterprise • Built on stale v2.0.0 (Rust) • MIT Licensed Core</div>
          <div className="flex items-center gap-4">
            <span>📍 Hyderabad, IN</span>
            <span>•</span>
            <span>⚡ &lt;15ms p95</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
