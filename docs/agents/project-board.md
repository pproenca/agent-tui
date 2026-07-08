# Project Board

GitHub Project: https://github.com/users/pproenca/projects/1

Use this Project as the progress dashboard for issues and external PRs in `pproenca/agent-tui`. GitHub Issues remain the source of truth for requirements and discussion; the Project is for workflow state, prioritization, and filtering.

The Project's **Auto-add to project** workflow is enabled for `agent-tui` with the filter `is:issue,pr is:open`, so new open issues and PRs are automatically added to the board.

## Fields

- **Status**: GitHub's built-in project status (`Todo`, `In Progress`, `Done`).
- **Workflow**: repo-specific triage state (`Inbox`, `Needs Info`, `Ready for Agent`, `Ready for Human`, `In Progress`, `In Review`, `Blocked`, `Done`).
- **Priority**: urgency and ordering (`P0`, `P1`, `P2`, `P3`).
- **Area**: user-facing or operational area (`CLI`, `Daemon`, `PTY`, `RPC/API`, `Web UI`, `Docs`, `Release`, `Architecture`).
- **Layer**: Clean Architecture owner (`common`, `domain`, `usecases`, `adapters`, `infra`, `app`, `facade`, `web`, `xtask`).
- **Size**: rough implementation size (`XS`, `S`, `M`, `L`).

## Saved Views

The Project has these saved views:

- **All Work**: https://github.com/users/pproenca/projects/1/views/1
- **Triage Inbox**: https://github.com/users/pproenca/projects/1/views/2 (`workflow:Inbox`)
- **Agent Queue**: https://github.com/users/pproenca/projects/1/views/3 (`workflow:"Ready for Agent"`)
- **Human Queue**: https://github.com/users/pproenca/projects/1/views/4 (`workflow:"Ready for Human"`)
- **Active Work**: https://github.com/users/pproenca/projects/1/views/5 (`workflow:"In Progress"`)
- **Review**: https://github.com/users/pproenca/projects/1/views/6 (`workflow:"In Review"`)
- **Architecture**: https://github.com/users/pproenca/projects/1/views/7 (`area:Architecture`)
- **Release**: https://github.com/users/pproenca/projects/1/views/8 (`milestone:*`)

## Workflow Mapping

- New issues should start with `needs-triage` and `Workflow: Inbox`.
- Issues waiting on the reporter use `needs-info` and `Workflow: Needs Info`.
- Fully specified agent-ready issues use `ready-for-agent` and `Workflow: Ready for Agent`.
- Work that needs a human uses `ready-for-human` and `Workflow: Ready for Human`.
- Active implementation uses `Status: In Progress` and `Workflow: In Progress`.
- Open PRs under review use `Workflow: In Review`.
- Closed issues and merged PRs use `Status: Done` and `Workflow: Done`.

When a skill creates or triages an issue, it should keep labels and Project workflow state in sync where the GitHub Project API is available.

GitHub's built-in **Item added to project** workflow sets `Status: Todo`. It does not set this repo's custom `Workflow` field, so agents should still set `Workflow` explicitly when applying triage labels.
