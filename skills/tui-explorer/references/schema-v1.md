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

Supported steps:
- `- expect: "<text>"`
- `- press: "<key>"`
- `- type: "<text>"`
- `- wait_stable: true`

Any other step format is invalid.
