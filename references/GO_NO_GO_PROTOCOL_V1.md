# Go/No-Go Protocol v1 — Reference Notes

## Product purpose

A brief, repeatable personal response-inhibition assessment. Not a clinical diagnostic instrument.

## Fixed parameters

- Total trials: 160.
- Trial composition: 75% Go, 25% No-Go.
- Stimulus mapping: fixed for a given protocol version.
- Trial order: seeded randomization, retained per session.
- Input: one configured keyboard key for Go response.
- Presentation and interval timings: fixed/versioned by implementation and retained in protocol metadata.

## Raw trial fields

- trial index
- stimulus kind (`go` / `no_go`)
- onset timestamp
- response timestamp if any
- reaction time if applicable
- correct/incorrect
- commission-error flag
- omission flag
- visibility/interruption state if detectable

## Derived metrics

- commission-error rate
- omission rate
- Go response-time median/mean
- valid-trial count
- optional signal-detection metrics only if implemented transparently and versioned

## Validity notes

A session can be completed but cautious/invalid due to interruption, visibility loss, incomplete trial count, input change, accidental key-hold detection, or user-reported issue.

## Practice effects

The app must display this as a repeated task with possible familiarization/practice effects. Early familiarization sessions should be tagged and excluded by default from primary trend analysis.
