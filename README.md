# MCC - Multi-Claude Code

Share Claude Code sessions between developers. Export your debugging session, send it to a teammate, they resume with full context.

```bash
# You
cd /my/project
mcc export fixing-auth-bug

# Teammate
cd /my/project
mcc import fixing-auth-bug
claude
/resume
```

## Install

```bash
cargo install --path . --features gcs
```

## Usage

```bash
# Export current session
mcc export my-session-name

# Import a session
mcc import my-session-name

# Browse all sessions (TUI)
mcc
```

## What Gets Shared

- Full conversation history
- All file changes and tool calls
- Git branch and working directory context

## Cloud Sharing (Optional)

```bash
# Setup
mcc config set-bucket gs://my-team-sessions
gcloud auth application-default login

# Share
mcc export my-fix
mcc share ~/.mcc/exports/my-fix.json.gz

# Fetch
mcc fetch gs://my-team-sessions/my-fix.json.gz
```

See [GCS_SETUP.md](GCS_SETUP.md) for details.

## Files

- Exports: `~/.mcc/exports/`
- Sessions: `~/.claude/projects/`

## Tips

- Export at debugging milestones
- Project structure should match between teammates
- Session includes git branch context

## License

MIT
