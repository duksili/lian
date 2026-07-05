# Acceptance Criteria

## Product foundation

- [ ] Application launches as a desktop app and remains usable without internet.
- [ ] Local data location is durable and documented.
- [ ] SQLite schema migration succeeds from an empty database.
- [ ] Backup, restore, and export workflows are available and testable.
- [ ] Data can be permanently deleted by the user.

## Daily tracking

- [ ] User can create a completed activity in under 15 seconds using a default template.
- [ ] User can run a timer and convert it into a completed activity.
- [ ] User can edit activity time, duration, details, note, and context after logging.
- [ ] User can create and reuse custom activity templates.
- [ ] Missing data is never automatically represented as zero or false.
- [ ] Timeline clearly distinguishes planned, completed, imported, and derived records.

## Daily check-in and Five Precepts

- [ ] User can configure a small set of daily state dimensions.
- [ ] Daily check-in stores date, entered time, selected ratings, and optional note.
- [ ] Five Precepts can be recorded daily using required statuses.
- [ ] Five Precepts appear as private reflection, not a score, streak, badge, or public metric.
- [ ] Five Precepts notes are not exposed in system notification text.

## Determinations

- [ ] User can create, edit, pause, revise, supersede, complete, and discontinue a voluntary determination without losing prior wording or history.
- [ ] A determination can be open-ended or time-bounded and can have an optional review cadence and neutral reminder.
- [ ] Determinations can link to plans, activity events, check-ins, and notes without automatic compliance inference.
- [ ] Determinations remain private and are never rendered as a morality score, streak, badge, ranking, or punitive status.

## Calendar, plans, and reminders

- [ ] User can create one-off and recurring plans.
- [ ] User can schedule custom activity/commitment types.
- [ ] User can link a completed activity to a plan explicitly.
- [ ] Editing future plans does not rewrite historical completion records.
- [ ] Reminders support enable/disable, quiet hours, snooze, and global pause.
- [ ] Missed/dismissed reminders do not create a failure status.
- [ ] Closing the main window can keep the app available through the system tray.

## Assessments

- [ ] PVT v1 meets its documented fixed protocol and stores raw trials.
- [ ] Go/No-Go v1 meets its documented fixed protocol and stores raw trials.
- [ ] Assessment sessions store protocol version, device/input metadata, validity state, and validity reasons.
- [ ] Interrupted/incomplete sessions can be retained and marked appropriately.
- [ ] Familiarization sessions can be excluded from primary analysis by default.
- [ ] Physical assessment records support the fields in the assessment document.

## Review and research

- [ ] Weekly review shows practice, plans, assessment completion, missingness, and context.
- [ ] Monthly review can inspect an exposure/outcome with a selected lag window.
- [ ] Analysis views display sample size, raw/inspectable observations, exclusions, missingness, metric version, and caveats.
- [ ] Analysis language uses only the defined evidence labels.
- [ ] User can create a structured protocol from a confirmed candidate hypothesis.
- [ ] Protocol results retain predefined question, outcome, schedule, and version.

## Visual quality and Fable freedom

- [ ] No acceptance criterion requires a specific dashboard layout, navigation pattern, visual theme, or component library.
- [ ] The implementation has a coherent interaction model chosen by Fable while preserving all semantic contracts.
- [ ] The finished product is visually polished and cohesive, with deliberate typography, spacing, hierarchy, states, and transitions rather than default-looking forms or disconnected pages.
- [ ] Core flows remain calm and immediately legible on compact, standard, and wide desktop windows; no essential action is hidden by overflow or weak contrast.
- [ ] Calendar, timeline, assessments, review, and charts are designed as first-class experiences rather than generic placeholders.
- [ ] Empty, loading, error, paused, and missing-data states are intentionally designed and use clear, non-punitive language.
- [ ] The product undergoes a dedicated visual refinement pass before delivery; functional completion alone is insufficient.
