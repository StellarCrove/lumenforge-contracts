# ADR-003: Two-Step Ownership Transfer

## Status

Accepted

## Context

A single `transfer_owner(new_owner)` call is the simplest way to hand off
a vault, but it has no recovery path: a mistyped address or a key the
new owner doesn't actually control permanently locks the vault, since
only `owner` can call privileged functions.

## Decision

Split the transfer into `propose_owner(new_owner)` (current owner names a
successor) and `accept_owner()` (the successor claims ownership by
authorizing the call themselves). Ownership only changes once the
successor proves control of the proposed address.

## Consequences

- One extra transaction for a full transfer, in exchange for eliminating
  a class of "sent to the wrong address" failures.
- `pending_owner()` is exposed as a view so callers/UIs can show a
  transfer in progress before it's accepted.
- The old owner retains full control (including the ability to cancel by
  proposing a different successor, or themselves) until `accept_owner`
  actually succeeds.
