# Quickstart — for Non-Developers

> You have an idea. Synapse + AI builds it for you.
> You never write code. You just describe what you want.

## 1. Install

```bash
curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.6.3/install.sh | sh
syn --version
```

## 2. Start a Project

```bash
mkdir my-app && cd my-app
syn init
opencode
```

## 3. Tell AI What You Want

```
You: "I want a task management app where I can create projects,
      add tasks, assign them to people, and track progress."

AI:  "Here's what I found:
      - Create projects with name and description
      - Add tasks with title, description, priority
      - Assign tasks to team members
      - Track status (todo, in-progress, done)
      - Dashboard showing project overview
      Does this look right? (yes/no/modify)"
```

## 4. Watch It Build

```
✓ Planning complete (5 modules identified)
✓ Module M-AUTH: authentication with JWT
✓ Module M-TASKS: CRUD for tasks
✓ Module M-PROJECTS: project management
✓ Module M-USERS: user profiles
✓ Module M-DASHBOARD: analytics view
✓ Verification passed (47 tests, 0 failures)
✓ Review passed (0 bugs, 0 security issues)
```

## 5. Make Changes

```
You: "Add dark mode and email notifications"

AI:  "I'll add two new requirements. Approve? (yes/no)"
```

## 6. When Something Breaks

```bash
syn skills run grace_fix issue="users can't log in after registration"
```

The helper returns a bounded fix path. In OpenCode, the AI can use the same `grace_fix`
workflow together with indexed code, `syn verify`, and `syn review`.

## What You NEVER Need to Do

| ❌ Never | ✅ Instead |
|----------|-----------|
| Write code | Describe what you want |
| Read documentation | Ask the AI |
| Debug errors | Use `grace_fix` through OpenCode or `syn skills run` |
| Run tests | `syn verify` does it |
| Review code | `syn review` does it |
| Configure build tools | `syn init` sets everything |
| Track progress | `syn status` shows it |

## Common Questions

**Do I need to know programming?**
No. You describe what you want in plain language.

**What if AI makes a mistake?**
Every module has contracts and tests. Mistakes are caught automatically.
If something slips through, use `grace_fix` through OpenCode or `syn skills run`.

**Is my code safe?**
Synapse runs locally and has no telemetry upload path. Your AI client may still send prompts or selected code context to whichever model provider you configure.
