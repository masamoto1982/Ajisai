# Task: bank-account

Ported task (mirrors the bank-account state exercises used in external
AI-language comparisons). Tests modelling deposits, withdrawals, and
overdraft rejection.

## Background (language-independent)

Model an account balance under deposits and withdrawals. A withdrawal that
would overdraw the account must be rejected rather than producing a negative
balance.

Ajisai has no mutable cells; state is threaded functionally as a value on
the stack. The idiomatic way to "reject" is the Bubble Rule: an invalid
withdrawal projects the balance to `NIL` (absence), and that `NIL` then
bubbles through any later operation — so a rejected account stays rejected.
This is the language-appropriate reading of "reject overdraft".

## Solution contract

Write an Ajisai source file that defines:

- `DEPOSIT` — given `[ balance ] [ amount ]`, leave `[ balance + amount ]`.
- `WITHDRAW` — given `[ balance ] [ amount ]`, leave `[ balance - amount ]`
  when that is ≥ 0, otherwise leave `NIL` (overdraft rejected). Hint: COND
  can branch on the sign of the computed new balance.

Balances are exact rationals, so fractional money is exact (no rounding).

## Acceptance cases (14)

| id | invocation | expected final stack |
|---|---|---|
| open-balance | `[ 0 ]` | `[ 0 ]` |
| deposit-once | `[ 0 ] [ 100 ] DEPOSIT` | `[ 100 ]` |
| deposit-twice | `[ 0 ] [ 100 ] DEPOSIT [ 50 ] DEPOSIT` | `[ 150 ]` |
| deposit-zero | `[ 50 ] [ 0 ] DEPOSIT` | `[ 50 ]` |
| withdraw-ok | `[ 100 ] [ 30 ] WITHDRAW` | `[ 70 ]` |
| withdraw-exact | `[ 100 ] [ 100 ] WITHDRAW` | `[ 0 ]` |
| withdraw-overdraft | `[ 50 ] [ 80 ] WITHDRAW` | `NIL` |
| dep-wd-chain | `[ 0 ] [ 100 ] DEPOSIT [ 30 ] WITHDRAW` | `[ 70 ]` |
| dep-wd-wd | `[ 0 ] [ 100 ] DEPOSIT [ 30 ] WITHDRAW [ 20 ] WITHDRAW` | `[ 50 ]` |
| overdraft-bubbles | `[ 50 ] [ 80 ] WITHDRAW [ 10 ] DEPOSIT` | `NIL` |
| withdraw-to-zero-then-over | `[ 100 ] [ 100 ] WITHDRAW [ 1 ] WITHDRAW` | `NIL` |
| deposit-fraction | `[ 0 ] [ 1 ] [ 2 ] / DEPOSIT` | `[ 1/2 ]` |
| withdraw-fraction | `[ 1 ] [ 1 ] [ 2 ] / WITHDRAW` | `[ 1/2 ]` |
| big-balance | `[ 0 ] [ 1000000000 ] DEPOSIT [ 1 ] DEPOSIT` | `[ 1000000001 ]` |

Display note: integers render as `[ n/1 ]`; the machine-readable `cases.tsv`
uses the exact display strings.

## Run

```sh
./verify.sh bank-account your-solution.ajisai
```
