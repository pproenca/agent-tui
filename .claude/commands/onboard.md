---
description: Interactive guided tour of agent-tui - see how AI agents control terminal apps
allowed-tools: AskUserQuestion, Bash, Read
---

# agent-tui Interactive Onboarding

Guide the user through an interactive demonstration of agent-tui's capabilities. This is a hands-on tour showing how AI agents can programmatically interact with terminal UI applications.

## Important Instructions

- Run each demo step and show the actual output
- After each major section, use AskUserQuestion to let the user continue when ready
- Be enthusiastic but concise - let the demos speak for themselves
- If any command fails, explain what happened and try to recover
- Clean up sessions at the end

---

## Section 1: Introduction

Start by explaining what they're about to see:

```
Welcome to agent-tui!

This tool lets AI agents interact with terminal UI applications - think of it as
"Playwright for the terminal."

You're about to see:
  1. The architecture (how it works under the hood)
  2. A simple bash demo (spawn, type, snapshot)
  3. A real TUI demo (top with element detection)
  4. The showstopper: AI controlling AI (Claude Code)

Let's dive in!
```

Use AskUserQuestion:
- question: "Ready to see the architecture?"
- header: "Continue"
- options:
  - "Let's go!" (Proceed with the tour)
  - "Skip to demos" (Jump straight to the hands-on part)

---

## Section 2: Architecture (if not skipped)

Show the architecture diagram:

```
How agent-tui Works
═══════════════════

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────┐         ┌──────────────────────────────────────────────┐  │
│  │   CLI       │         │              DAEMON (Native Rust)            │  │
│  │  (Rust)     │ JSON-RPC│                                              │  │
│  │             │ ──────► │  ┌─────────────┐    ┌─────────────────────┐  │  │
│  │ • spawn     │ Unix    │  │ IPC Server  │───►│  Session Manager    │  │  │
│  │ • snapshot  │ Socket  │  │ Routes reqs │    │                     │  │  │
│  │ • click     │         │  └─────────────┘    │ Sessions Map:       │  │  │
│  │ • fill      │ ◄────── │                     │  "abc123" → {       │  │  │
│  │ • keystroke │ Response│                     │    pty,             │  │  │
│  │ • wait      │         │                     │    terminal,        │  │  │
│  │ • kill      │         │                     │    running: true    │  │  │
│  └─────────────┘         │                     │  }                  │  │  │
│                          │                     └──────────┬──────────┘  │  │
│                          │                                │             │  │
│                          │                     ┌──────────▼──────────┐  │  │
│                          │                     │   For each session  │  │  │
│                          │                     │                     │  │  │
│                          │  ┌────────────────┐ │ ┌─────────────────┐ │  │  │
│                          │  │  Native PTY    │ │ │ Virtual Terminal│ │  │  │
│                          │  │                │ │ │                 │ │  │  │
│                          │  │ Real PTY that  │◄┼─│ Parses ANSI,    │ │  │  │
│                          │  │ runs the app   │ │ │ maintains screen│ │  │  │
│                          │  │                │─┼►│ buffer          │ │  │  │
│                          │  │   ┌────────┐   │ │ └─────────────────┘ │  │  │
│                          │  │   │  htop  │   │ │                     │  │  │
│                          │  │   │  vim   │   │ └─────────────────────┘  │  │
│                          │  │   │  etc   │   │                          │  │
│                          │  └────────────────┘                          │  │
│                          └──────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

The magic: A native Rust terminal emulator maintains an in-memory screen buffer.
It interprets ANSI escape codes so AI can read the terminal state at any time.
```

Use AskUserQuestion:
- question: "Ready to see it in action?"
- header: "Continue"
- options:
  - "Show me!" (Proceed to demos)

---

## Section 3: Demo 1 - Simple Bash Session

Explain what you're about to do:

```
Demo 1: Basic Interaction
═════════════════════════

Let's spawn a bash session and interact with it programmatically.
Watch how we: spawn → type → snapshot → execute → see results
```

### Step 3.1: Spawn bash

Run: `./cli/target/release/agent-tui spawn bash`

Show the output and explain:
- A session ID was created (like "abc12345")
- A real bash process is running in a virtual terminal
- The daemon is managing the PTY

### Step 3.2: Type a command

Run: `./cli/target/release/agent-tui type "echo 'Hello from agent-tui!'"`

Then take a snapshot: `./cli/target/release/agent-tui snapshot`

Show the output - the typed text appears on the virtual terminal screen.

### Step 3.3: Press Enter and see the result

Run: `./cli/target/release/agent-tui keystroke Enter`

Wait briefly, then snapshot: `sleep 0.3 && ./cli/target/release/agent-tui snapshot`

Show the output - the command executed and "Hello from agent-tui!" appeared!

### Step 3.4: Clean up

Run: `./cli/target/release/agent-tui kill`

Use AskUserQuestion:
- question: "That was the basics! Ready for something more impressive?"
- header: "Continue"
- options:
  - "Bring it on!" (Continue to Demo 2)
  - "Replay that" (Go back to Demo 1)

---

## Section 4: Demo 2 - Real TUI with Element Detection

```
Demo 2: Element Detection
═════════════════════════

Now let's interact with a real TUI application: `top` (process monitor).
Watch how agent-tui detects interactive elements automatically!
```

### Step 4.1: Spawn top

Run: `./cli/target/release/agent-tui spawn top`

Wait for it to start: `sleep 0.5`

### Step 4.2: Take a snapshot WITH element detection

Run: `./cli/target/release/agent-tui snapshot -i`

Show the output and highlight:
- The screen content (process list, CPU usage, etc.)
- The detected elements with sequential refs like @e1, @e2, @e3
- Each element has a type, label, position, and state

Explain:
```
Notice the element refs like @e1, @e2, @e3 - these are sequential identifiers
assigned in document order (top-to-bottom, left-to-right). Refs reset on each
snapshot, so always use the latest snapshot's refs!
```

### Step 4.3: Interact with top

Press 'q' to quit: `./cli/target/release/agent-tui keystroke q`

Then snapshot to show it exited: `sleep 0.2 && ./cli/target/release/agent-tui snapshot`

Clean up: `./cli/target/release/agent-tui kill 2>/dev/null`

Use AskUserQuestion:
- question: "Now for the grand finale - something mind-blowing. Ready?"
- header: "Continue"
- options:
  - "I'm ready!" (Continue to Demo 3)

---

## Section 5: Demo 3 - The Showstopper (Claude Code)

```
Demo 3: AI Controlling AI
═════════════════════════

This is where it gets meta. We're going to spawn Claude Code itself
and control it programmatically. An AI controlling an AI's interface!

         ┌─────────────────────────────────────────┐
         │  You (watching)                         │
         │         ↓                               │
         │  agent-tui (this demo)                  │
         │         ↓                               │
         │  Claude Code (another AI instance)      │
         │         ↓                               │
         │  Real answers!                          │
         └─────────────────────────────────────────┘
```

### Step 5.1: Spawn Claude Code

Run: `./cli/target/release/agent-tui spawn claude -- --dangerously-skip-permissions`

Wait for it to start: `sleep 2`

Take a snapshot: `./cli/target/release/agent-tui snapshot`

Show the Claude Code welcome screen with the logo!

### Step 5.2: Ask Claude a question

Type a question: `./cli/target/release/agent-tui type "What is 2+2?"`

Snapshot to show it: `./cli/target/release/agent-tui snapshot`

Press Enter: `./cli/target/release/agent-tui keystroke Enter`

Wait for response: `sleep 3`

Snapshot: `./cli/target/release/agent-tui snapshot`

Show the response - Claude answered "4"!

### Step 5.3: Ask something about the codebase

Type: `./cli/target/release/agent-tui type "How many Rust files are in this project? Just the count."`

Enter: `./cli/target/release/agent-tui keystroke Enter`

Wait: `sleep 5`

Snapshot: `./cli/target/release/agent-tui snapshot`

Show Claude using the Search tool and answering!

### Step 5.4: Use the wait command

Explain: "Now watch the `wait` command - it waits for specific text to appear."

Type: `./cli/target/release/agent-tui type "Say hello in Spanish"`

Enter: `./cli/target/release/agent-tui keystroke Enter`

Wait for "Hola": `./cli/target/release/agent-tui wait "Hola" -t 10000`

Show that it waited until "Hola" appeared, reporting the exact time.

Snapshot to show the full result: `./cli/target/release/agent-tui snapshot`

### Step 5.5: Clean up

Exit Claude: `./cli/target/release/agent-tui type "/exit" && ./cli/target/release/agent-tui keystroke Enter`

Wait: `sleep 1`

Clean up: `./cli/target/release/agent-tui cleanup --all`

---

## Section 6: Wrap-up

```
That's agent-tui!
═════════════════

What you just saw:

  ✓ Spawning terminal applications in virtual terminals
  ✓ Reading screen content with snapshots
  ✓ Detecting interactive UI elements automatically
  ✓ Sending keystrokes and text
  ✓ Waiting for specific content to appear
  ✓ An AI controlling another AI's interface!

Use Cases:
  • Automated testing of TUI applications
  • AI agents that can run CLI wizards (npm init, create-react-app, etc.)
  • Scripted interactions with interactive tools
  • Building meta-AI systems that orchestrate other tools

Core Commands:
  spawn <cmd>     Start an app in virtual terminal
  snapshot -i     See screen + detected elements
  type "text"     Type literal text
  keystroke Key   Send key (Enter, Tab, Ctrl+C, etc.)
  click @e1       Click/activate an element
  fill @e1 "val"  Fill input with value
  wait "text"     Wait for text to appear
  kill            End session

Advanced Commands:
  find --text "Submit"     Find elements by text content
  find --role button       Find elements by role (button, input, etc.)
  wait --stable            Wait for screen to stabilize
  scroll down              Scroll the terminal
  toggle @e1               Toggle checkbox/radio
  restart                  Restart session (like reload)
  cleanup --all            Clean up all sessions

Happy automating!
```

Use AskUserQuestion:
- question: "Would you like to explore more?"
- header: "Next"
- options:
  - "Run /help" (Show available commands)
  - "I'm done" (End the tour)
  - "Show me the code" (Show key source files)

If "Show me the code":
  Show the paths to key files:
  - `cli/src/main.rs` - CLI entry point
  - `cli/src/daemon/mod.rs` - Native daemon entry point
  - `cli/src/daemon/session.rs` - Session management
  - `cli/src/daemon/terminal.rs` - Terminal emulation

If "Run /help":
  Run: `./cli/target/release/agent-tui --help`
