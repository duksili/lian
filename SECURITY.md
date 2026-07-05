# Security and Privacy

## Data sensitivity

LIAN stores private practice, reflection, assessment, scheduling, and potentially health-adjacent context data locally. Treat all runtime data, backups, exports, screenshots, and logs as sensitive by default.

## Repository rules

Never commit:

- user SQLite databases, WAL/SHM files, backups, or exports;
- private notes, Five Precepts reflections, determination details, assessment results, or screenshots containing personal data;
- credentials, signing keys, `.env` files, or local service tokens.

Use deliberately sanitized fixtures in `fixtures/` or `testdata/` when test data must be version-controlled.

## Reporting a security issue

Do not place a suspected vulnerability or sensitive sample data in a public issue or shared discussion. Report it privately to the repository owner through the established private channel, with a minimal reproducible description and no real personal dataset.

## v1 security posture

- Core use is local-first and must not depend on network access.
- The application should minimize sensitive content in notifications and lock-screen text.
- Backups and exports remain under user control.
- Any future sync, telemetry, or external integration requires a separate documented privacy and security review before implementation.
