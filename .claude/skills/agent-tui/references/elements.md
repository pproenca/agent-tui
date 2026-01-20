# agent-tui Element Types

agent-tui detects 13 types of interactive UI elements in TUI applications.

## Element Types

| Type | Ref Pattern | Description | Actions |
|------|-------------|-------------|---------|
| **Button** | `@btn1`, `@btn2` | Clickable buttons | `click` |
| **Input** | `@inp1`, `@inp2` | Text input fields | `fill`, `clear`, `focus` |
| **Checkbox** | `@cb1`, `@cb2` | Checkbox controls | `toggle`, `click` |
| **Radio** | `@rb1`, `@rb2` | Radio button options | `click` |
| **Select** | `@sel1`, `@sel2` | Dropdown/select menus | `select` |
| **MenuItem** | `@mi1`, `@mi2` | Menu items | `click` |
| **ListItem** | `@li1`, `@li2` | List items | `click` |
| **Link** | `@lnk1`, `@lnk2` | Hyperlinks | `click` |
| **Spinner** | `@spn1` | Loading spinners | (read-only) |
| **Progress** | `@prg1` | Progress bars | (read-only) |
| **Text** | `@txt1` | Static text blocks | `get-text` |
| **Container** | `@cnt1` | Container elements | (grouping) |
| **Unknown** | `@unk1` | Unclassified elements | varies |

## Element Properties

Each element has the following properties:

```json
{
  "ref": "@btn1",
  "type": "button",
  "label": "Submit",
  "value": null,
  "position": { "row": 5, "col": 10, "width": 8, "height": 1 },
  "focused": true,
  "selected": false,
  "checked": null,
  "disabled": false,
  "hint": "Press Enter to submit",
  "options": null
}
```

| Property | Description |
|----------|-------------|
| `ref` | Stable identifier for interactions |
| `type` | Element type (button, input, etc.) |
| `label` | Display text/label |
| `value` | Current value (for inputs) |
| `position` | Screen location (row, col, width, height) |
| `focused` | Whether element has focus |
| `selected` | Whether element is selected |
| `checked` | For checkboxes/radios |
| `disabled` | Whether element is disabled |
| `hint` | Usage hint if available |
| `options` | For select elements, list of options |

## Tree Format Output

With `--format tree`, elements display as:

```
- button "Submit" [ref=@btn1, focused]
- input "Project name" [ref=@inp1] value="my-app"
- checkbox "Use TypeScript" [ref=@cb1, checked]
- select "Package manager" [ref=@sel1] value="npm"
- radio "License" [ref=@rb1, selected]
```

## Finding Elements

### By Ref (after snapshot)
```bash
agent-tui click @btn1
agent-tui fill @inp1 "value"
```

### Semantically (without snapshot)
```bash
agent-tui find --role button --name "Submit"
agent-tui find --text "Continue"
agent-tui find --focused
```

## Element Stability

Element refs are **content-based** to prevent drift:
- Same visual content = same ref across snapshots
- UI changes may shift refs (re-snapshot before interacting)
- Refs are scoped per session

## Best Practices

1. **Take fresh snapshots** before interacting with elements
2. **Use semantic find** when refs might have changed
3. **Check element type** before actions (don't `fill` a button)
4. **Handle focus** - some elements need focus before input
5. **Use wait** after actions that change the UI
