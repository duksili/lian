# PVT Protocol v1 — Reference Notes

## Product purpose

A brief, repeatable personal sustained-attention assessment. Not a clinical diagnostic instrument.

## Fixed parameters

- Duration: 5 minutes.
- Interstimulus interval: randomized 2–10 seconds.
- Input: one configured keyboard key.
- Stimulus: clear visual target/counter; onset timestamp recorded.
- False start: response before onset or response under 100 ms.
- Lapse: response time >= 500 ms.
- Session seed: persisted to support reproducibility of sequence generation.

## Raw trial fields

- trial index
- planned interval
- actual stimulus onset
- response timestamp
- reaction time
- false-start flag
- lapse flag
- no-response/timeout flag
- visibility/interruption state if detectable

## Derived metrics

- median reaction time
- mean reaction time
- lapse count and rate
- false-start count
- response-time variability
- valid-trial count

Derived metric calculations must be versioned.

## Suggested standard conditions

The user should choose a preferred comparable condition, for example: seated at the same computer, before caffeine, during a chosen morning window. The app records deviations rather than blocking all variation.

## Validity notes

A session can be completed but receive a caution/invalid flag for interruption, hidden window, wrong input method, incomplete duration, excessive false starts, or a user-reported issue.
