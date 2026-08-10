# Security

## Threat Model

### Assets at risk

- Tokens/native balance represented by the `BALANCE` ledger value.
- Availability of `withdraw` for the legitimate owner.

### Trusted parties

- The address passed to `initialize` is fully trusted with the entire
  balance. There is no recovery mechanism if that key is lost or
  compromised.

## Authentication

- `deposit` requires `from.require_auth()` — a caller can only deposit on
  behalf of an address that has authorized the invocation.
- `withdraw` requires `owner.require_auth()`, where `owner` is read from
  storage rather than taken as a caller-supplied argument. This prevents a
  caller from claiming ownership by simply passing their own address.

## Reentrancy

Soroban's execution model does not permit the classic EVM-style reentrancy
pattern (no fallback-triggered external calls mid-execution), so this is
not treated as a primary risk. State (`BALANCE`) is updated before the
function returns in both `deposit` and `withdraw`, consistent with
checks-effects-interactions as a defensive default.

## Arithmetic

- `i128` is used for the balance to avoid the overflow ranges of smaller
  integer types at realistic token amounts.
- `deposit` and `withdraw` both reject non-positive amounts.
- `withdraw` rejects amounts greater than the current balance, preventing
  underflow.
- Overflow on `deposit` (balance + amount) is not currently checked
  explicitly; it relies on the Soroban host's panic-on-overflow behavior
  for arithmetic operations in debug builds. See Known Limitations.

## Known Limitations

### 1. `initialize` is not access-controlled

Any address can call `initialize` before the legitimate deployer does,
becoming the owner. Deployers must call `initialize` in the same
transaction/flow as contract creation, or otherwise ensure no other
transaction can front-run it.

### 2. No per-depositor accounting

`BALANCE` is a single pooled value. If multiple addresses deposit, the
contract has no on-chain record of who contributed what — only the owner
can withdraw, and only in aggregate. This is a design choice
([ADR-001](adr/001-single-balance-vault.md)), not an oversight, but it
means this contract is unsuitable for use cases requiring per-depositor
withdrawal rights without an additional access layer.

### 3. No overflow check on deposit accumulation

`balance + amount` in `deposit` is not wrapped in a `checked_add`. At
`i128` range this is not practically reachable with real token supplies,
but it should be made explicit before a mainnet audit.

### 4. No pause / emergency-stop mechanism

There is no way to freeze deposits or withdrawals if a vulnerability is
discovered post-deployment short of the owner withdrawing the full
balance.

## Disclosure

This project has not yet had an external audit. If you find a
vulnerability, please open a private security advisory on this repository
rather than a public issue.

## Audit Checklist (pre-mainnet)

- [ ] Access control on `initialize`
- [ ] Explicit checked arithmetic on `deposit`
- [ ] Decide on and document per-depositor accounting requirements
- [ ] Third-party audit of `contracts/lumen_vault`
- [ ] Testnet soak period with monitored deposits/withdrawals
