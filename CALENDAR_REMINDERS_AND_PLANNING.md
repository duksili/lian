# Calendar, Reminders, and Planning

## Planning model

Planning records intention. Completed activity records reality. They must remain distinct.

A plan can represent:

- scheduled practice;
- exercise or movement;
- an assessment;
- recovery/rest;
- a custom personal commitment;
- a research protocol requirement.
- an optional concrete occurrence linked to a determination.

A plan may be one-off or recurring. Changing future plans must not rewrite prior occurrences or completed records.

## Plan status

Suggested derived states:

- `upcoming`
- `due`
- `completed_linked`
- `completed_unlinked`
- `skipped`
- `cancelled`
- `expired_unresolved`

A skipped, cancelled, or unresolved plan is not a failure signal. It is information about the plan.

## Plan-to-actual linking

A completed activity may be linked to one plan. A plan may have multiple actual links only where explicitly allowed, such as a split session. The UI may suggest links based on timing/template, but must not silently create them.

## Calendar behavior

The user must be able to:

- view planned activity by day, week, and month;
- create/edit/delete one-off plans;
- set recurrence;
- drag or otherwise reschedule future plans;
- distinguish plans from completed activity;
- open a plan and see linked completion;
- include custom plans not represented by an activity template;
- turn a plan into a completed event only through an explicit action that preserves both records.

No external calendar integration is required in v1.

## Determinations and plans

A determination may be associated with zero or more plans, but it is not a plan occurrence. Completing, skipping, or cancelling a plan must not silently set a determination review status. A determination may instead have an explicit review cadence that asks for private reflection.

## Reminder types

- Scheduled reminder: at a specific time or before/after a planned activity.
- Window reminder: available during a protocol or assessment window.
- Event-triggered reminder: optional follow-up after logging activity.
- Review reminder: weekly/monthly review.
- Recovery reminder: optional prompt for yesterday’s unreviewed data.
- Determination review reminder: optional, neutral prompt to review an active determination.

## Reminder safeguards

- Quiet hours are mandatory and configurable.
- Reminders must be individually disableable and snoozable.
- The application must support a global pause mode.
- A missed or dismissed reminder must not generate a “failure”.
- The notification text must be neutral and factual.
- Notifications should not disclose private Five Precepts or determination details on a lock screen.
- The system must avoid bursts: no stacked catch-up notices after downtime.

## Recommended default notification behavior

- At most one non-critical reminder at a time.
- Assessment reminders occur inside a chosen eligible window.
- Activity reminders can be sent before planned start and optionally at start.
- Post-practice check-in prompt is opt-in and brief.
- Weekly review is offered once, then remains available without repeated pressure.
