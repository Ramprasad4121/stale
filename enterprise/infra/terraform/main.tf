terraform {
  required_version = ">= 1.0"
  required_providers {
    stale = {
      source  = "stale-enterprise/stale"
      version = "~> 1.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.11"
    }
  }
}

# Variables
variable "stale_api_key" {
  description = "Stale Enterprise API key (stale_live_...)"
  type        = string
  sensitive   = true
}

variable "environment" {
  description = "Environment: production, staging, development"
  type        = string
  default     = "production"
}

variable "org_slug" {
  description = "Organization slug"
  type        = string
  default     = "acme-defi"
}

# Provider
provider "stale" {
  api_key     = var.stale_api_key
  environment = var.environment
  base_url    = "https://api.stale.sh"
}

# Data source: existing org
data "stale_organization" "current" {
  slug = var.org_slug
}

# Production strict policy
resource "stale_policy" "production_strict" {
  name        = "Production - Strict"
  description = "Fail-closed strict policy for production trading"
  environment = "production"
  is_default  = true

  rules = {
    fail_mode         = "fail_closed"
    guard_timeout_ms  = 5000
    max_guards        = 32

    oracle = {
      enabled                  = true
      max_age_seconds          = 60
      feeds                    = ["ETH/USD", "BTC/USD"]
      require_deviation_check  = true
      max_deviation_bps        = 100
      fallback_feeds           = ["ETH/USD:Arbitrum"]
    }

    gas = {
      enabled                = true
      max_gas_gwei           = 50
      max_priority_fee_gwei  = 3
      max_base_fee_gwei      = 80
      block_on_high_gas      = true
    }

    network = {
      enabled                  = true
      allowed_chain_ids        = [1, 10, 42161, 8453]
      require_mev_protection   = true
      allowed_rpc_hosts        = ["rpc.flashbots.net", "ethereum-rpc.publicnode.com"]
      check_sequencer          = true
      sequencer_grace_seconds  = 3600
      check_rpc_sync           = true
      max_rpc_drift_seconds    = 60
    }

    permissions = {
      enforce_allowlist                = true
      allowlist = {
        UNISWAP_V3_ROUTER = "0xE592427A0AEce92De3Edee1F18E0157C05861564"
        WETH              = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
      }
      block_unbounded_approvals  = true
      check_allowance            = true
      check_balance              = true
      check_is_contract          = true
      rate_limit_per_min         = 60
      spending_cap_usd_per_hour  = 100000
    }

    compliance = {
      check_sanctions       = true
      require_audit_log     = true
      block_paused_contracts = true
      check_honeypot        = true
    }
  }
}

# High-value policy for large transactions
resource "stale_policy" "high_value" {
  name        = "High-Value - Fort Knox"
  description = "Maximum security for transactions > $50k"
  environment = "production"
  is_default  = false

  rules = {
    fail_mode         = "fail_closed"
    guard_timeout_ms  = 10000
    max_guards        = 64

    oracle = {
      enabled                  = true
      max_age_seconds          = 30
      feeds                    = ["ETH/USD", "BTC/USD", "USDC/USD"]
      require_deviation_check  = true
      max_deviation_bps        = 50
    }

    gas = {
      enabled                = true
      max_gas_gwei           = 30
      max_priority_fee_gwei  = 2
      max_base_fee_gwei      = 50
      block_on_high_gas      = true
    }

    permissions = {
      enforce_allowlist                = true
      allowlist = {
        UNISWAP_V3_ROUTER = "0xE592427A0AEce92De3Edee1F18E0157C05861564"
      }
      rate_limit_per_min         = 10
      spending_cap_usd_per_hour  = 10000
    }
  }
}

# API Keys
resource "stale_api_key" "trading_bot_prod" {
  name                = "Production - Trading Bot"
  environment         = "production"
  scopes              = ["check_read", "check_write", "pipeline_run", "audit_read"]
  rate_limit_per_min  = 1000
  ip_allowlist        = ["10.0.0.0/8", "54.123.45.67/32"] # your prod IPs
}

resource "stale_api_key" "ci_staging" {
  name        = "Staging - CI/CD"
  environment = "staging"
  scopes      = ["check_read", "pipeline_run"]
}

# Webhook for BLOCK alerts
resource "stale_webhook" "slack_alerts" {
  url         = "https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"
  environment = "production"
  events      = ["guardrail.blocked"]
  secret      = "whsec_..." # for signing
}

# Outputs
output "production_policy_id" {
  value = stale_policy.production_strict.id
}

output "trading_bot_api_key_prefix" {
  value = stale_api_key.trading_bot_prod.key_prefix
}

output "dashboard_url" {
  value = "https://app.stale.sh/orgs/${var.org_slug}/policies/${stale_policy.production_strict.id}"
}
