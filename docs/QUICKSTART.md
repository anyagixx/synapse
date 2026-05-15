# Quickstart — for Non-Developers

> You have an idea. Synapse + AI builds it for you.
> You never write code. You just describe what you want.

## 1. Install

```bash
curl -fsSL https://synapse.dev/install.sh | sh
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
syn fix "users can't log in after registration"
```

AI finds the bug through the knowledge graph, reads the relevant code,
fixes it, and verifies the fix.

## What You NEVER Need to Do

| ❌ Never | ✅ Instead |
|----------|-----------|
| Write code | Describe what you want |
| Read documentation | Ask the AI |
| Debug errors | Run `syn fix` |
| Run tests | `syn verify` does it |
| Review code | `syn review` does it |
| Configure build tools | `syn init` sets everything |
| Track progress | `syn status` shows it |

## Common Questions

**Do I need to know programming?**
No. You describe what you want in plain language.

**What if AI makes a mistake?**
Every module has contracts and tests. Mistakes are caught automatically.
If something slips through, `syn fix` corrects it.

**Is my code safe?**
Yes. All data stays on your computer. Synapse runs locally.
No code is sent to external servers (unless you choose cloud embedding).
