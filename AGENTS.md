# LIAN — Implementation Rules for Agents

## Start here

Read, in order:

1. `README.md`
2. `FABLE_IMPLEMENTATION_HANDOFF.md`
3. `PRODUCT_CHARTER.md`
4. `SCOPE_AND_NON_GOALS.md`
5. `DOMAIN_AND_DATA_CONTRACTS.md`
6. the relevant file in `contracts/`
7. the relevant acceptance criteria and protocol documents

The document-authority order in `README.md` is binding when two documents appear to conflict.

## Non-negotiable product rules

- Keep LIAN local-first in v1. Do not introduce mandatory accounts, cloud storage, or network dependencies.
- Treat missing data as `unknown`; do not infer missed practice, non-observance, or failed determination review.
- Keep planned activities, completed activities, Five Precepts reflection, determinations, assessments, and research protocols as distinct records.
- Preserve stable IDs, event time, log time, provenance, edit metadata, protocol versions, and assessment validity state.
- Store raw assessment trial/attempt data where required; do not retain only summary scores.
- Keep Five Precepts and determinations private, non-punitive, and free of scores, streaks, badges, rankings, or moral language.
- Analysis must distinguish observation, association, candidate hypothesis, and protocol result. Never claim causation from observational data.
- Notifications must be configurable, quiet, privacy-safe, and non-punitive.

## Change discipline

- Do not change a contract silently. Update the relevant contract and acceptance criteria in the same change, or record a specific conflict in `references/OPEN_QUESTIONS.md`.
- Do not commit personal databases, backups, exports, screenshots containing personal data, credentials, or signing keys.
- Prefer the smallest coherent implementation that preserves the frozen product meaning.
- Do not add a feature merely because a library makes it easy. Check `SCOPE_AND_NON_GOALS.md` first.
- Keep visual design open and high-quality. Do not substitute a generic admin dashboard for intentional product design.

## Completion evidence

For each meaningful delivery, provide source changes, migration changes where applicable, focused test evidence, a mapping to acceptance criteria, and unresolved questions or deviations.
