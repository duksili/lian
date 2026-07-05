# Daily Tracking and Practice Model

## Design objective

Daily tracking must be rich enough for future review but light enough to remain usable on tired, busy, ill, or unmotivated days.

A user should be able to log a practice at three levels:

1. **Fast:** activity + duration.
2. **Normal:** activity + duration + selected structured fields.
3. **Rich:** normal entry + note/context.

No level is morally preferred.

## Required initial activity templates

- Meditation
- Taiji
- Yoga / mobility
- Walking
- Strength / sport / cardio
- Recovery / rest
- Custom activity

The user must be able to add, rename, archive, and reorder templates without losing historical records.

## Suggested structured fields

Fields are optional unless a relevant protocol requires them.

| Field | Applies to |
|---|---|
| Duration | Most activities |
| Start/end time | Most activities |
| Practice subtype | Meditation and Taiji especially |
| Intensity | Movement/exercise activities |
| Perceived quality | Any deliberate practice |
| Before/after body state | Taiji, yoga, mobility, sport |
| Location/context | Optional for all |
| Note | Optional for all |
| Plan link | Optional for all |

### Taiji suggested subtypes

- form
- standing
- silk-reeling
- applications / partner work
- correction / study
- class
- mixed
- custom

### Meditation suggested subtypes

- seated
- walking
- guided
- chanting / recitation
- informal mindfulness
- retreat / extended
- custom

## Daily check-in

The user selects a small set of dimensions during onboarding. Suggested choices:

- calm
- energy
- focus
- body tension
- mood
- recovery
- sleepiness
- pain/stiffness

Each dimension uses a fixed ordinal scale, recommended 1–5. Labels and anchors must be shown consistently. The app must not collapse these into an opaque composite score.

## Sleep and common context

A daily check-in may include:

- sleep start and wake time;
- estimated duration;
- subjective quality;
- awakenings if the user chooses;
- caffeine timing/amount if logged;
- illness/injury or other tags.

Sleep data must remain explicitly self-reported unless a future imported source is identified.

## Context events and life anchors

The user must be able to create low-friction context events such as:

- illness began/ended;
- injury/stiffness;
- travel;
- unusual work load;
- emotional stress;
- significant change in routine;
- practice change;
- sleep disruption;
- custom event.

A context event can span a time range, have tags, and appear in timeline/review overlays.

## Five Precepts

Five Precepts reflection is a separate daily record, not an activity and not a score.

Requirements:

- It is private, optional, and visually unobtrusive.
- It should normally be accessed from the daily/evening check-in or review.
- Each precept has a daily state: `observed`, `not_observed`, `uncertain`, or `not_reviewed`.
- The user may add an optional private note.
- The application must not show moral scores, public badges, streaks, punishments, or prescriptive judgments.
- Review may show frequency and personal notes only.

See `contracts/five-precepts.contract.md`.


## Determinations

A determination is a voluntary, user-defined commitment or resolution that the user wants to hold over time. Examples include a temporary practice emphasis, a conduct intention, a restraint, or a personal commitment. It is **not** the same thing as a calendar plan: a plan schedules a concrete occurrence, while a determination describes an ongoing intention that may have many related plans or none. It is also distinct from the Five Precepts.

Requirements:

- A determination has a title, optional description, start date, and explicit lifecycle.
- It may be open-ended or time-bounded.
- It may have an optional review cadence and neutral reminder.
- It may link to plans, activity events, daily check-ins, and context notes, but those links must not silently prove compliance.
- The user can mark a review as `kept`, `not_kept`, `uncertain`, or `not_reviewed` only when the determination has a review rule; otherwise it remains a narrative commitment.
- The user can pause, revise, supersede, complete, or discontinue a determination without losing historical wording or creating a failure score.
- Determinations must remain private, visually calm, and free of streaks, badges, rankings, shame language, or aggregate moral scoring.

See `contracts/determination.contract.md`.
