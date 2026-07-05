# Assessments and Validity Rules

## Purpose

Assessments provide repeatable personal outcome measures. They are not clinical tests, diagnoses, or intelligence measures.

The implementation must prioritize protocol consistency, raw data retention, test-version visibility, and transparent invalidation over attractive score presentation.

## Assessment roster for v1

1. PVT v1 — primary sustained-attention/alertness assessment.
2. Go/No-Go v1 — response inhibition assessment.
3. Weekly physical assessment — single-leg stance and five-times sit-to-stand.

No other cognitive test is required in v1.

## Common session requirements

Every assessment session must record:

- assessment kind and protocol version;
- session status;
- start/end timestamps;
- local timezone;
- device/platform metadata where available;
- input method;
- pre-test condition fields defined by protocol;
- interruption/visibility state where detectable;
- validity status and validity reasons;
- raw trials/attempts where the protocol contains trials;
- derived metrics version.

An assessment may be completed but marked invalid or cautionary. Invalid data must remain viewable and exportable; it must not silently disappear.

## PVT v1

Reference implementation requirements:

- Fixed duration: 5 minutes.
- Interstimulus interval: randomized, 2–10 seconds.
- Visual stimulus: a clear counter or target that begins at stimulus onset.
- Primary input: one consistent keyboard key.
- False start: response before stimulus onset or response under 100 ms.
- Lapse threshold: response time at or above 500 ms.
- Store each stimulus onset, response timestamp, reaction time, false start/lapse flags, and any omitted trial.
- Display neutral completion feedback; do not present diagnostic claims.

Recommended validity warnings:

- app lost foreground/visibility during active test;
- keyboard/input method differs from the configured method;
- excessive false starts;
- incomplete duration;
- user marked interruption;
- test taken outside usual configured window;
- protocol version changed.

## Go/No-Go v1

Reference implementation requirements:

- Fixed, versioned visual stimulus protocol.
- Recommended initial configuration: 160 trials, 75% Go and 25% No-Go.
- Stimuli and target mapping must be randomized under a reproducible session seed.
- Store per-trial stimulus type, onset, response, response time, correctness, commission error, omission, and trial ordering.
- Summary metrics include commission-error rate, omission rate, Go response time, and valid-trial count.
- Do not compare results across protocol versions as if they were identical.

Recommended validity warnings:

- lost foreground/visibility;
- incomplete trial count;
- input-method change;
- repeated accidental key hold/detection issue;
- self-reported interruption;
- protocol condition mismatch.

## Weekly physical assessment

The app must support structured recording of:

### Single-leg stance

- left/right side;
- attempt duration;
- max configured cap;
- number of attempts;
- support/touchdown event;
- footwear/surface/context optional note.

### Five-times sit-to-stand

- total completion time;
- start/finish confirmation;
- chair/condition note optional;
- whether pain or balance concern occurred.

The app should provide safety language: stop if pain, dizziness, instability, or concern occurs. It must not assess fall risk or health status.

## Baseline and familiarization

- Initial familiarization sessions should be tagged and excluded from primary trend/association output by default.
- The baseline period must be visible and configurable.
- Changes to protocol, equipment, input method, or assessment schedule must be recorded as context for later interpretation.

## Display rules

- Show raw data availability and validity state.
- Prefer personal range/trend views to normative labeling.
- Avoid scores such as “brain age,” “mental fitness,” “attention grade,” or medical interpretation.
