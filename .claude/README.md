# Skill Detector — Auto-discover reusable patterns

A Claude Code hook that quietly evaluates every session after it finishes and
logs suggestions for new skills when it spots a reusable, multi-step pattern.
Suggestions accumulate in a file. You review them on your own schedule.

## How it works

```
You finish a task
       │
       ▼
  Stop hook fires (async, non-blocking)
       │
       ▼
  skill-detector.py reads the transcript
       │
       ▼
  Calls Haiku to evaluate:
  "Was this a reusable multi-step pattern
   not covered by an existing skill?"
       │
       ├── No  → exits silently
       │
       └── Yes → appends suggestion to
                 ~/.claude/skill-suggestions.json
       
Later, you run /review-skill-ideas
       │
       ▼
  Claude presents pending suggestions
  You create, dismiss, or skip each one
```

## Setup

### 1. Copy the hook script

```bash
mkdir -p ~/.claude/hooks
cp hooks/skill-detector.py ~/.claude/hooks/
chmod +x ~/.claude/hooks/skill-detector.py
```

### 2. Copy the review skill

```bash
cp -r skills/review-skill-ideas ~/.claude/skills/
```

### 3. Add the hook to your settings

Merge the contents of `settings-snippet.json` into your
`~/.claude/settings.json`. If you already have a `"hooks"` key, add the
`"Stop"` entry alongside your existing hooks.

The key config:

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "python3 ~/.claude/hooks/skill-detector.py",
            "async": true,
            "timeout": 20
          }
        ]
      }
    ]
  }
}
```

**`async: true`** is critical — it means the hook runs in the background
and never blocks you from getting your response.

### 4. Make sure ANTHROPIC_API_KEY is in your environment

The hook calls the Anthropic API (Haiku) to evaluate transcripts.
If the key isn't set, the hook exits silently and does nothing.

```bash
# e.g. in your shell profile:
export ANTHROPIC_API_KEY="sk-ant-..."
```

## Usage

Just use Claude Code normally. After sessions with substantive multi-step
work, the hook will silently log suggestions. Then:

```
/review-skill-ideas
```

Or just ask: "Are there any skill suggestions to review?"

## Tuning

**Too many suggestions?** Increase `MIN_TURNS` in `skill-detector.py`
(default: 4). A higher threshold means only longer, more complex sessions
get evaluated.

**Too few?** Lower `MIN_TURNS` or increase `MAX_TRANSCRIPT_CHARS` to give
the evaluator more context.

**Want project-level detection only?** Move the hook config from
`~/.claude/settings.json` to `.claude/settings.json` in specific repos.

## Files

```
hooks/
  skill-detector.py      — The async Stop hook (evaluates transcripts)
skills/
  review-skill-ideas/
    SKILL.md              — Skill for reviewing & acting on suggestions
settings-snippet.json     — Hook config to merge into your settings
```
