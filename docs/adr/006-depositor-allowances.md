# ADR 006: Optional depositor allowances

## Status

Accepted. Implemented in `lumen_vault` 0.6.0 (`contracts/lumen_vault/src/allowance.rs`).

## Context

A vault's `deposit` is permissionless apart from `from.require_auth()` and the pause, minimum, and maximum checks. That is the right default for a personal vault the owner funds themselves. An institutional vault is different: the owner wants a payroll or a treasury counterparty to deposit up to a named amount, and nobody else.

Tracking each deposit as a separate liability (per-depositor accounting, ADR-001) is a different product. The owner can already withdraw the whole balance. What was missing is a cap on who may add funds, and how much.

## Decision

Allowances are off unless the owner calls `set_allowances_enabled(true)`.

- Off: `deposit` and `batch_deposit` behave as before. Vaults deployed earlier never wrote this key, and a missing key reads as off.
- On: `from` must have a remaining allowance of at least `amount`. A successful deposit reduces that remaining amount. It does not record a claim on the pool. Only the owner can still withdraw.
- `set_allowance(depositor, amount)` is owner-only. `amount == 0` removes the address and frees a slot. A negative amount is `InvalidAmount`.
- The member list lives in instance storage and is capped at `MAX_ALLOWANCES` (32). Past that, a new address returns `AllowanceListFull`. Replacing an existing member's remaining amount does not take a new slot.
- The check runs before the SEP-41 `transfer`. If the transfer later fails the exact-credit check, the allowance write reverts with the invocation.

## Consequences

- Instance storage grows by one address plus one `i128` per member, and one `Vec` of at most 32 addresses. `extend_ttl` on the vault still covers all of it, because none of these keys are persistent.
- Enabling allowances on a vault that already holds funds does not lock those funds. Withdrawal is unchanged. It only gates further deposits.
- A depositor with a leftover allowance can still deposit after the owner turns the switch on. The owner should set amounts, or clear the list, before enabling if the previous open period should not carry over. Amounts can be set while the switch is off; they have no effect until it is on.
- This is not a spending approval. The depositor cannot withdraw against their allowance.

## Alternatives considered

- A separate policy contract the vault calls. That adds an external call on every deposit and a second address to keep alive. The list is small enough to live on the vault.
- Minting a receipt token per deposit. That is per-depositor accounting, which ADR-001 rejected for this contract.
