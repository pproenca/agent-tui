# Navigation Commands

Browser navigation commands and their TUI applicability.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `navigate` | N/A | ❌ N/A | No URL navigation in TUI |
| `back` | N/A | ❌ N/A | No browser history |
| `forward` | N/A | ❌ N/A | No browser history |
| `reload` | `spawn` (re-run) | ⚠️ Different | Kill + respawn |
| `url` | `get-title` | ⚠️ Different | Returns command instead |
| `title` | `get-title` | ⚠️ Partial | Returns session command/title |

---

## Why Navigation Commands Don't Apply to TUI

TUI applications run in a terminal PTY and don't have the concept of:

1. **URLs** - Terminal apps don't navigate to addresses
2. **History** - No browser-style back/forward navigation
3. **Page loads** - No equivalent to page reload

### Equivalent Patterns

For scenarios where you might use navigation in a browser, here are TUI equivalents:

| Browser Pattern | TUI Equivalent |
|-----------------|----------------|
| Navigate to URL | `spawn` new application |
| Reload page | `kill` + `spawn` (restart app) |
| Get current URL | `sessions` (get command) |
| Back/Forward | App-specific keystrokes (if supported) |

---

## navigate (Browser only)

**Purpose**: Navigate to a URL

### Browser Signature

```typescript
interface NavigateCommand {
  action: 'navigate';
  url: string;
  waitUntil?: 'load' | 'domcontentloaded' | 'networkidle';
  timeout?: number;
}

// Response
interface NavigateResponse {
  navigated: boolean;
  url: string;
  status: number;
}
```

### TUI Equivalent

❌ **Not applicable** - TUI apps don't have URLs.

**Alternative**: To "navigate" to different functionality in a TUI app, use:
- `keystroke` to send navigation keys
- `click` to activate menu items
- `fill` to enter commands in shell prompts

---

## back / forward (Browser only)

**Purpose**: Navigate browser history

### Browser Signature

```typescript
interface BackCommand {
  action: 'back';
}

interface ForwardCommand {
  action: 'forward';
}
```

### TUI Equivalent

❌ **Not applicable** - No history stack in TUI.

**Alternative**: Some TUI apps have their own navigation history:
- `keystroke "Alt+Left"` - Some apps use this for back
- `keystroke "Escape"` - Often exits to previous screen
- App-specific commands

---

## reload (Browser) → kill + spawn (TUI)

**Purpose**: Restart the application

### Browser Signature

```typescript
interface ReloadCommand {
  action: 'reload';
  ignoreCache?: boolean;
}

// Response
interface ReloadResponse {
  reloaded: boolean;
}
```

### TUI Equivalent Pattern

```bash
# Get session info
agent-tui sessions

# Kill and respawn
agent-tui kill --session htop-abc123
agent-tui spawn htop
```

### Notes

- TUI has no single "reload" command
- Must kill existing session and spawn new one
- Consider adding a `restart` convenience command

---

## url (Browser) → sessions (TUI)

**Purpose**: Get current location

### Browser Signature

```typescript
interface UrlCommand {
  action: 'url';
}

// Response
interface UrlResponse {
  url: string;
}
```

### TUI Equivalent

Use `sessions` to get the command being run:

```bash
agent-tui sessions
# Returns: { "sessions": [{ "id": "...", "command": "htop", ... }] }
```

---

## title (Browser) → get-title (TUI)

**Purpose**: Get current title

### Browser Signature

```typescript
interface TitleCommand {
  action: 'title';
}

// Response
interface TitleResponse {
  title: string;
}
```

### TUI Signature

```rust
pub struct GetTitleParams {
    pub session: Option<String>,
}

pub struct GetTitleResult {
    pub title: String,
    pub session_id: String,
}
```

### Notes

- Browser returns the HTML `<title>` element
- TUI returns the terminal title (set by apps via escape sequences) or falls back to command name
- Not all TUI apps set a terminal title

---

## Summary

Navigation commands are fundamentally browser concepts. For TUI automation:

1. **Start apps** with `spawn`
2. **Restart apps** with `kill` + `spawn`
3. **Navigate within apps** using `keystroke`, `click`, `fill`
4. **Get current state** with `sessions` or `snapshot`
