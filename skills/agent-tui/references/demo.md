# Demo Mode: The "Wow" Experience

## Philosophy: Show, Don't Tell

When someone asks "What does agent-tui do?", don't describe it—**demonstrate it**. A 60-second live demo is worth 1000 words of explanation.

The goal: User sees a browser tab open, watches a real terminal app launch, sees the agent identify elements, press keys, and verify outcomes—all in real-time.

## The Demo Sequence

### Pre-flight Check

Before starting, verify the setup:

```bash
agent-tui --version  # Confirm installed
```

If not installed, guide the user through installation first (see main skill).

### Step 1: Start Live Preview (First "Wow")

```bash
agent-tui live start --open
```

**What happens:** A browser tab opens showing a live terminal view. This is the first visual "wow"—they can watch everything you do.

**Say:** "I'm opening a live preview so you can watch in real-time."

### Step 2: Launch a Dynamic App

```bash
agent-tui run top
```

**What happens:** The `top` command launches, showing real-time system processes with CPU, memory, and process data updating continuously.

**Say:** "This is `top`—a system monitor with constantly updating data. Perfect for showing real-time detection."

**Why `top`:** Universal on macOS/Linux, dynamic content, no installation needed, impressive visual complexity.

### Step 3: Element Detection (The Magic)

```bash
agent-tui screenshot -e --format json
```

**What happens:** Returns JSON with detected UI elements—headers, process rows, labels, all with element refs like `@e1`, `@e2`.

**Say:** "I can see [X] elements—process names, CPU percentages, headers. Each has a unique reference I can interact with."

**Pro tip:** Call out specific elements you see: "The 'kernel_task' process is using X% CPU right now."

### Step 4: Interaction (Prove Control)

```bash
agent-tui press q
```

**What happens:** Sends the 'q' key to quit `top`. The app exits cleanly.

**Say:** "I just sent 'q' to quit. Watch the display change."

### Step 5: Verification (The Clincher)

```bash
agent-tui screenshot
```

**What happens:** Shows the terminal returned to a clean state (shell prompt or exit message).

**Say:** "The app exited cleanly. I can verify any state—not just assume it worked."

### Step 6: Clean Exit

```bash
agent-tui kill
agent-tui live stop
```

**What happens:** Session ends, live preview closes.

**Say:** "Always clean up. No orphaned processes, no resource leaks."

## Complete Demo Script (Copy-Paste Ready)

```bash
# Full demo sequence
agent-tui live start --open
agent-tui run top
agent-tui wait --stable
agent-tui screenshot -e --format json
agent-tui press q
agent-tui wait --stable
agent-tui screenshot
agent-tui kill
agent-tui live stop
```

## Alternative Demo Apps

If `top` isn't appropriate, consider:

| App | Command | Why | Quit Key |
|-----|---------|-----|----------|
| `top` | `agent-tui run top` | Dynamic, universal, impressive | `q` |
| `htop` | `agent-tui run htop` | More visual, if installed | `q` |
| `vim` | `agent-tui run vim` | Ubiquitous, shows modal UI | `:q!` then Enter |
| `nano` | `agent-tui run nano` | Simple, common | `Ctrl+X` |
| `less` | `agent-tui run less /etc/hosts` | Read-only, safe | `q` |

## Failure Handling During Demo

| Problem | Solution |
|---------|----------|
| Live preview doesn't open | Check browser, try `agent-tui live start` without `--open`, give URL manually |
| `top` not found | Use `htop`, `vim`, or `less /etc/hosts` instead |
| Elements not detected | Wait longer (`wait --stable`), resize terminal (`resize --cols 120 --rows 40`) |
| App won't quit | `agent-tui kill` forces cleanup |
| Browser tab blank | Refresh, or restart with `agent-tui live stop && agent-tui live start --open` |

## After the Demo: Transition Prompts

Once the demo completes, offer next steps:

- "What would you like to automate? I can help you script any terminal app."
- "Want to try this with your own app? Just tell me what command to run."
- "I can also show you how to write tests that verify TUI behavior."
- "Any questions about what you just saw?"

## Timing Guide

| Step | Duration |
|------|----------|
| Live preview open | ~5 seconds |
| Launch `top` | ~3 seconds |
| Element detection | ~5 seconds |
| Narrate findings | ~20 seconds |
| Send quit, verify | ~10 seconds |
| Cleanup | ~5 seconds |
| **Total** | **~50-60 seconds** |

## Demo Mindset

- **Confidence is key:** You're showing off. Be impressed by your own capabilities.
- **Narrate everything:** Don't just run commands silently. Explain what's happening.
- **Point out specifics:** "I see 47 processes" is better than "I see some processes."
- **Make it interactive:** Ask if they want you to try something specific.
- **Handle failures gracefully:** If something breaks, fix it calmly and continue.

---

*This demo is designed to create a "wow" moment. The user should walk away thinking "this is magical" rather than "this is complicated."*
