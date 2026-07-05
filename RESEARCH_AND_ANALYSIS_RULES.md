# Research and Analysis Rules

## Core principle

The app begins as an observational longitudinal record. Observational patterns can generate hypotheses; they do not prove causation.

## Review layers

| Layer | Purpose |
|---|---|
| Daily | Capture and timeline clarity |
| Weekly | Adherence, missingness, context, reflection |
| Monthly | Trends, descriptive summaries, candidate associations |
| Protocol review | Predefined hypothesis and outcome assessment |

## Baseline phase

The default initial baseline is 4–6 weeks, configurable by the user. During baseline:

- show completion and data-quality status;
- show descriptive trends;
- do not generate causal-looking insights;
- label any early relationship as insufficient evidence.

## Association analysis requirements

A user must be able to define:

- exposure: e.g., Taiji duration, meditation, walking distance, sleep duration;
- outcome: e.g., next-morning PVT lapse rate, tension rating, Go/No-Go commission errors;
- time window/lag: same day, next day, two days, custom;
- comparison band: none/short/medium/long where meaningful;
- context filters or stratification: sleep, caffeine, illness, work load, weekday, protocol validity;
- excluded data: invalid assessments, familiarization sessions, user-selected periods.

Every analysis view must show:

- sample count;
- included and excluded observations;
- missing-data count;
- raw records or inspectable points;
- definition/version of derived metric;
- caveats;
- evidence label;
- generated timestamp.

## Evidence labels

Use only these labels in v1:

- `descriptive`: summary without a tested relationship.
- `insufficient_data`: not enough comparable observations.
- `observational_signal`: apparent association worth monitoring.
- `candidate_hypothesis`: repeated observational signal that justifies a predefined protocol.
- `protocol_result_inconclusive`: protocol completed without a clear result.
- `protocol_result_supported`: result consistent with the predefined protocol hypothesis, not a universal proof.
- `protocol_result_not_supported`: result not consistent with the predefined protocol hypothesis.

Do not use “caused”, “proves”, “works”, “improves”, “heals”, or similar causal language unless a future explicitly designed methodology supports it.

## Candidate-hypothesis requirements

A candidate hypothesis should require:

- sufficient observations, with threshold shown rather than hidden;
- repeat pattern across more than one time period where possible;
- review of obvious confounders;
- no unresolved major data-quality warning;
- explicit human confirmation before protocol creation.

## Research protocol rules

A protocol must define before analysis:

- question/hypothesis;
- intervention/exposure;
- primary outcome;
- schedule and duration;
- expected lag/carryover;
- adherence requirements;
- key context variables;
- analysis window;
- stop/pause criteria;
- conclusion state.

The app must not silently adjust the outcome after seeing results without creating a new protocol version.

## Interpretation safeguards

- Correlation may reflect reverse causation, common causes, selection effects, time trends, or missing variables.
- Poor sleep, illness, travel, routine disruption, testing time, caffeine, and workload should be easy to inspect as context.
- Small sample sizes should be surfaced, not hidden behind charts.
- Analysis should preserve negative, null, and inconclusive results.
- The user may export raw data for external analysis at any time.
