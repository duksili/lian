# Core User Flows

This document defines outcomes and behavior. It intentionally does not prescribe screen count, layout, navigation style, component design, or visual identity.

## 1. First-run setup

The user must be able to:

1. Understand that the product is local-first and private.
2. Choose which daily state dimensions to track.
3. Enable, disable, or customize activity templates.
4. Set a home timezone and quiet hours.
5. Select initial assessment schedules and preferred assessment windows.
6. Create the first backup destination or explicitly defer it.
7. Enter a baseline period with no causal or predictive claims.

The application should avoid forcing extensive setup. Defaults must be editable later.

## 2. Open application / today state

When the application is opened, the user must be able to quickly understand:

- what is planned or due today;
- what was recently recorded;
- whether a scheduled assessment is available;
- which determinations are currently active or due for review;
- whether yesterday has unreviewed or missing entries;
- a direct path to record a practice, context event, quick note, or check-in.

The visual representation is Fable’s responsibility.

## 3. Quick log completed practice

The user must be able to create a completed activity in roughly 5–15 seconds with:

- activity type;
- occurred time, defaulting to now;
- duration or a timer-based duration;
- optional intensity and optional note.

The user must be able to add richer details later without re-creating the event.

## 4. Daily state and Five Precepts reflection

The user must be able to record a short daily state check-in. The Five Precepts record must be available but visually quiet: collapsible, optional, and non-punitive.

The app must support an evening check-in reminder, but must not convert it into a streak or failure system.

## 5. Determinations

The user must be able to create a voluntary personal determination with a title, optional description, date range or open-ended duration, optional review cadence, and optional neutral reminder.

The user must be able to:

- keep an active determination visible without it dominating daily use;
- link it to related plans or evidence without automatic compliance inference;
- review it using the documented non-punitive statuses when applicable;
- pause, revise, supersede, complete, or discontinue it while retaining historic wording and review history;
- inspect it privately across time.

A determination is distinct from a scheduled plan and from Five Precepts reflection.

## 6. Calendar planning

The user must be able to:

- add one-off or recurring plans;
- plan exercise, practice, assessment, recovery, or custom commitments;
- optionally associate a plan with an active determination;
- set reminder preferences;
- change a plan without changing past completed events;
- link a completed activity to a plan when appropriate;
- see planned versus completed status without treating an unfulfilled plan as a moral failure.

## 7. Scheduled assessment

When an assessment is due, the user must be able to:

1. See the standardized conditions/instructions.
2. Start, cancel, pause only where the protocol allows, or defer.
3. Complete the test with protocol-consistent behavior.
4. Have the result stored with raw observations and validity metadata.
5. See a neutral completion result, not a gamified score.
6. Add context that may affect validity.

## 8. Weekly review

The user must be able to review:

- practice volume and distribution;
- planned versus completed activity;
- assessment completion and validity;
- missing or uncertain data;
- Five Precepts reflection history in a private, non-scored form;
- active and reviewed determinations in a private, non-scored form;
- life events and context tags;
- optional weekly reflection.

The review should invite annotation before memory fades.

## 9. Monthly research review

The user must be able to select an exposure, outcome, lag window, and relevant context filters, then inspect:

- how many observations are involved;
- raw distributions or records;
- missingness;
- evidence language;
- caveats and confounders;
- whether the result is an observation, association, candidate hypothesis, or protocol result.

## 10. Correction, deletion, backup, export

The user must be able to correct recorded data, delete it, export it, and restore from a backup. Past analysis results must retain enough provenance to show when they were generated and from which data version where practical.
