/**
 * @module addressbook
 * Address Book / Allowlist Guardrail.
 *
 * A configurable allowlist of known-good contract addresses the agent is
 * permitted to interact with. Any address NOT in the book is blocked.
 *
 * This is the simplest, most powerful defense: if you haven't explicitly
 * approved a contract, the agent cannot touch it.
 */

export type AddressBookResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type AddressBookConfig = {
  /** Map of label → address (checksummed or lowercase, both accepted) */
  allowlist: Record<string, string>;
  /** If true, block all addresses not in the allowlist (default: true) */
  strict?: boolean;
};

/**
 * In-memory address allowlist.
 * Configure with known-good router addresses, token addresses, etc.
 */
export class AddressBook {
  private readonly addresses: Map<string, string>; // lowercase address → label
  private readonly strict: boolean;

  constructor(config: AddressBookConfig) {
    this.strict = config.strict ?? true;
    this.addresses = new Map();

    for (const [label, addr] of Object.entries(config.allowlist)) {
      if (!/^0x[a-fA-F0-9]{40}$/.test(addr)) {
        throw new Error(`invalid address for "${label}": ${addr}`);
      }
      this.addresses.set(addr.toLowerCase(), label);
    }
  }

  /**
   * Check if a target address is in the allowlist.
   */
  check(address: string): AddressBookResult {
    if (!/^0x[a-fA-F0-9]{40}$/.test(address)) {
      return {
        decision: "BLOCK",
        reason: `invalid address ${address} — BLOCK`,
        allowExecute: false,
      };
    }

    const normalized = address.toLowerCase();
    const label = this.addresses.get(normalized);

    if (label) {
      return {
        decision: "ALLOW",
        reason: `address is allowlisted as "${label}"`,
        allowExecute: true,
      };
    }

    if (this.strict) {
      return {
        decision: "BLOCK",
        reason: `UNKNOWN ADDRESS: ${address} is NOT in the allowlist. The agent cannot interact with unknown contracts. — BLOCK`,
        allowExecute: false,
      };
    }

    return {
      decision: "ALLOW",
      reason: `address not in allowlist but strict mode is off`,
      allowExecute: true,
    };
  }

  /** Check if an address is known */
  has(address: string): boolean {
    return this.addresses.has(address.toLowerCase());
  }

  /** Get the label for an address */
  labelOf(address: string): string | undefined {
    return this.addresses.get(address.toLowerCase());
  }

  /** Number of entries */
  size(): number {
    return this.addresses.size;
  }
}
