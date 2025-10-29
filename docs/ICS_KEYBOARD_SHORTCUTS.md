# ICS Keyboard Shortcuts

## Integrated Context Studio - Complete Keyboard Reference

### Editor Navigation

| Shortcut | Action |
|----------|--------|
| `←` / `→` | Move cursor left/right |
| `↑` / `↓` | Move cursor up/down |
| `Home` | Jump to line start |
| `End` | Jump to line end |

### Editing

| Shortcut | Action |
|----------|--------|
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Enter` | Insert newline |
| `Ctrl+Z` | Undo last change |
| `Ctrl+Y` | Redo last undone change |
| `Ctrl+S` | Save current file |

### Panel Toggles

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Ctrl+M` | Toggle Memory Panel | Show/hide relevant memories from Mnemosyne |
| `Ctrl+P` | Toggle Proposals Panel | Show/hide agent change proposals |
| `Ctrl+D` | Toggle Diagnostics Panel | Show/hide validation errors and warnings |
| `Ctrl+A` | Toggle Agent Status Panel | Show/hide active agents and their activities |
| `Ctrl+T` | Toggle Attribution Panel | Show/hide change attribution (who made each edit) |

### Application Control

| Shortcut | Action |
|----------|--------|
| `Ctrl+Q` | Quit ICS |
| `Ctrl+C` | Quit ICS (alternative) |

## Panel-Specific Features

### Memory Panel (`Ctrl+M`)
- Browse memories relevant to current context
- See memory importance scores
- View memory connections

### Proposals Panel (`Ctrl+P`)
- Review agent-suggested changes
- See change rationale and diff
- Accept (`a`) or reject (`r`) proposals (when implemented)
- Filter by status: Pending, Accepted, Rejected, Applied

### Diagnostics Panel (`Ctrl+D`)
- View all validation issues
- Navigate to problem locations
- See error counts by severity:
  - ✗ Errors (red)
  - ⚠ Warnings (yellow)
  - ● Hints (blue)

### Agent Status Panel (`Ctrl+A`)
- Monitor active agents
- See current agent activities
- Track agent collaboration

### Attribution Panel (`Ctrl+T`)
- See who made each change
- Distinguish human vs. agent edits:
  - Human authors: Green
  - Agent authors: Purple
- View change timeline

## Inline Indicators

### Gutter Icons
Appear next to line numbers for lines with diagnostics:
- `✗` Error (soft red)
- `⚠` Warning (soft yellow)
- `●` Hint (soft blue-gray)

### Text Decorations
- **Underlined text**: Has diagnostic issue
- **Colored text**: Shows change attribution (when CRDT enabled)

## UI Philosophy

ICS follows "calm technology" principles:
- **Progressive disclosure**: All panels hidden by default
- **Muted colors**: RGB values in 140-200 range
- **Non-intrusive**: Diagnostics are soft suggestions, not blocking errors
- **Subtle feedback**: Visual indicators without distraction

## Status Bar

Bottom status bar shows:
- Current line and column position
- Document language
- Semantic analysis stats (when available):
  - Triple count
  - Typed hole count
  - Entity count

## Tips

1. **Start minimal**: Open ICS with no panels, focus on writing
2. **Toggle as needed**: Show panels only when you need them
3. **Trust the gutter**: Diagnostic icons show issues without opening panel
4. **Save often**: `Ctrl+S` commits changes
5. **Undo freely**: `Ctrl+Z` for unlimited undo history

## Future Shortcuts (Planned)

- `Ctrl+F`: Find in document
- `Ctrl+H`: Find and replace
- `Ctrl+/`: Toggle comment
- `Ctrl+K`: Command palette
- `F8`: Next diagnostic
- `Shift+F8`: Previous diagnostic
- `Ctrl+.`: Quick fix menu

---

*ICS is part of Mnemosyne - AI-native project memory system*
