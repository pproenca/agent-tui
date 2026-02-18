# Schema v1

Acceptance files are markdown with YAML frontmatter.

## Required frontmatter
- `schema_version: "v1"`
- `command`
- `cols`
- `rows`
- `default_timeout_ms`
- `generated_at`
- `generator`

Optional:
- `cwd`

## Scenario format
Scenario header:
- `## Scenario: <name>`

Optional OpenSpec-style expectation narrative (human-readable):
- `### Expectation` (or `### Expectations`)
- `- **WHEN** ...`
- `- **THEN** ...`
- `- **AND** ...` (optional)
- `- **SHOULD** ...`

Supported steps:
- `- expect: "<text>"`
- `- press: "<key>"`
- `- type: "<text>"`
- `- wait_stable: true`

OpenSpec narrative lines are ignored by the verifier parser; executable behavior is driven by the supported step lines.
Any other non-empty line format is invalid.
