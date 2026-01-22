# Agent-TUI Command Specifications

This directory contains specifications mapping agent-browser commands to their TUI equivalents.

## Overview

| Metric | Count |
|--------|-------|
| Total agent-browser commands analyzed | 147 |
| TUI-applicable commands | 45 |
| Already implemented in agent-tui | 32 |
| Missing (should add) | 8 |
| Not applicable (browser-only) | 102 |

## Specification Files

| File | Category | Commands |
|------|----------|----------|
| [01-SESSION_LIFECYCLE.spec.md](01-SESSION_LIFECYCLE.spec.md) | Session Lifecycle | spawn, kill, sessions, health |
| [02-NAVIGATION.spec.md](02-NAVIGATION.spec.md) | Navigation | N/A for TUI (browser URLs) |
| [03-ELEMENT_INTERACTION.spec.md](03-ELEMENT_INTERACTION.spec.md) | Element Interaction | click, fill, type, focus, clear |
| [04-SNAPSHOT.spec.md](04-SNAPSHOT.spec.md) | Snapshot | snapshot |
| [05-KEYBOARD_MOUSE.spec.md](05-KEYBOARD_MOUSE.spec.md) | Keyboard & Mouse | keystroke, scroll |
| [06-WAITING.spec.md](06-WAITING.spec.md) | Waiting | wait, waitfor* |
| [07-STATE_QUERY.spec.md](07-STATE_QUERY.spec.md) | State Query | get_text, is_visible |
| [08-FORM_ELEMENTS.spec.md](08-FORM_ELEMENTS.spec.md) | Form Elements | toggle, select |
| [09-SEMANTIC_LOCATORS.spec.md](09-SEMANTIC_LOCATORS.spec.md) | Semantic Locators | find |
| [10-RECORDING.spec.md](10-RECORDING.spec.md) | Recording | record_start, trace, console |
| [11-SESSION_MANAGEMENT.spec.md](11-SESSION_MANAGEMENT.spec.md) | Session Management | resize, attach |

## TUI Applicability Matrix

### Legend
- ✅ **Exists** - Implemented with matching semantics
- ⚠️ **Different** - Implemented but with different approach
- ❌ **Missing** - Should add to TUI
- ❌ **N/A** - Browser-only, no TUI equivalent

### Command Status Summary

| Browser Command | TUI Equivalent | Status |
|-----------------|----------------|--------|
| launch | spawn | ✅ Exists |
| close | kill | ✅ Exists |
| tab_list | sessions | ✅ Exists |
| click | click | ✅ Exists |
| dblclick | - | ❌ Missing |
| focus | focus | ✅ Exists |
| hover | - | ❌ N/A |
| type | type | ✅ Exists |
| fill | fill | ✅ Exists |
| press | keystroke | ✅ Exists |
| clear | clear | ✅ Exists |
| snapshot | snapshot | ✅ Exists |
| screenshot | snapshot --strip-ansi | ✅ Exists |
| keyboard | keystroke | ✅ Exists |
| scroll | scroll | ✅ Exists |
| wait | wait | ✅ Exists |
| gettext | get_text | ✅ Exists |
| inputvalue | get_value | ✅ Exists |
| isvisible | is_visible | ✅ Exists |
| isenabled | - | ❌ Missing |
| ischecked | - | ❌ Missing |
| check/uncheck | toggle | ✅ Exists |
| select | select | ✅ Exists |
| getbyrole | find --role | ✅ Exists |
| getbytext | find --text | ✅ Exists |
| recording_start | record_start | ✅ Exists |
| trace_start | trace --start | ✅ Exists |
| console | console | ✅ Exists |
| viewport | resize | ✅ Exists |
| bringtofront | attach | ✅ Exists |

## Commands with No TUI Equivalent

These browser commands have no TUI equivalent:

1. **Navigation**: navigate, back, forward, url (URLs don't exist in TUI)
2. **Mouse-only**: hover, drag, mousemove, mousedown, mouseup
3. **Browser APIs**: cookies_*, storage_*, evaluate, expose, addscript, addstyle
4. **Network**: route, unroute, requests, responsebody, offline, headers
5. **DOM**: setcontent, innerhtml, getattribute, styles, boundingbox
6. **Media**: emulatemedia, pdf, download
7. **Emulation**: geolocation, permissions, timezone, locale, credentials, device, useragent
8. **Dialogs**: dialog (browser native dialogs)
9. **Debugging**: highlight, pause (browser inspector)
10. **CDP**: screencast_*, input_mouse, input_keyboard, input_touch

## Missing TUI Commands (Should Add)

| Priority | Command | Description |
|----------|---------|-------------|
| High | dblclick | Double click (send Enter twice) |
| High | isenabled | Check if element is disabled |
| High | ischecked | Check checkbox/radio state |
| High | count | Count matching elements |
| Medium | multiselect | Multiple selection in lists |
| Medium | nth | Select nth matching element |
| Medium | errors | Collect stderr/error output |
| Medium | keydown/keyup | Hold/release modifier keys |

## Implementation Plan

### Phase 1: Verify Existing Parity
- spawn, kill, sessions, health
- click, fill, type, keystroke, focus, clear
- snapshot, screen
- wait (all conditions)
- scroll, scrollintoview
- get_text, get_value, is_visible, is_focused
- toggle, select
- find (semantic locator)
- record_start, record_stop, trace, console
- resize, attach

### Phase 2: Add Missing Commands
1. Add `dblclick` command
2. Add `isenabled` / `is_enabled` command
3. Add `ischecked` / `is_checked` command
4. Add `count` command

### Phase 3: Enhanced Functionality
5. Add `multiselect` for multi-option selection
6. Add `nth` parameter to find
7. Add `errors` command for stderr capture
8. Add `keydown`/`keyup` for modifier key hold

### Phase 4: Polish
9. Add `exact` parameter to find
10. Document all command equivalences

## Critical Files

- `/cli/src/commands.rs` - CLI command definitions
- `/cli/src/protocol.rs` - JSON-RPC protocol types
- `/cli/src/daemon/` - Command handlers
