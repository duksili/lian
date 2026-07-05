# Contributing to LIAN

LIAN is a private, local-first personal practice and research application. The product documents in this repository are the implementation baseline.

## Before changing behavior

1. Read the authority order in `README.md`.
2. Identify the affected user flow, contract, and acceptance criteria.
3. Keep planning separate from completion, and keep personal reflection non-punitive.
4. Preserve privacy and data provenance.
5. Record a concrete question in `references/OPEN_QUESTIONS.md` when a requirement is ambiguous rather than silently deciding it.

## Required change quality

- Keep schema and migration changes explicit and reversible where practical.
- Add focused tests for data integrity, lifecycle rules, assessment logic, and reminder behavior affected by the change.
- Do not commit real personal data, SQLite databases, exports, logs, credentials, or signing material.
- Do not weaken the distinction between association and causation.
- Do not add cloud, social, medical, gamified, or opaque-AI behavior outside the documented scope.
- Keep the desktop experience polished and coherent; functional completion alone is not sufficient.

## Review checklist

A contribution is ready for review when it explains:

- what changed and why;
- which product contract or acceptance criteria it satisfies;
- migration and compatibility impact;
- focused test/manual verification evidence;
- unresolved edge cases, deferrals, or document updates.
