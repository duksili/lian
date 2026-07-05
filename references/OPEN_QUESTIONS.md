# Open Questions

Use this file for true unresolved design choices, implementation conflicts, or decisions requiring user input. Do not use it as a vague backlog.

## Current open questions

1. **Assessment schedule defaults:** Should first-run defaults be PVT three mornings weekly and Go/No-Go twice weekly, or should all schedules begin disabled until the user opts in?
2. **Physical assessment UX:** Should the weekly physical check be guided with a timer/voice prompts, or should it begin as manual result entry with instructions?
3. **Data encryption at rest:** Is OS-level account protection sufficient for v1, or should application-level encrypted database support be planned early?
4. **Backups:** Should automatic scheduled backup be a v1 hard requirement or a v1.1 feature after manual backup/restore is proven?
5. **Assessment window behavior:** Should a due assessment disappear after its window, become “late but available,” or remain available with a validity deviation flag?

## Decision record format

For future entries:

- Question:
- Why it matters:
- Options:
- Recommended option:
- Decision owner:
- Date decided:
- Result:


## Determinations

The v1 contract treats a determination as a voluntary user-defined commitment distinct from plans and the Five Precepts. The implementation must preserve the distinction. Any desired specialized Buddhist terminology or additional canonical determination categories should be added only through a later explicit product decision.
