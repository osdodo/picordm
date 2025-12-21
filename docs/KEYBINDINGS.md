# Quick Reference - Keybindings

## Global Shortcuts
| Key | Function |
|-----|----------|
| `Ctrl+q` | Exit application |

## Features
- ✅ Vim-style editing (Normal/Insert/Visual modes)
- ✅ JSON syntax highlighting (auto-detect)
- ✅ Vim commands (:w, :q, :wq, :q!)
- ✅ Copy/paste (system clipboard)
- ✅ Undo/redo
- ✅ Mouse support

## Dashboard Mode
| Key | Function |
|-----|----------|
| `↑↓` | Select key |
| `Enter` | Open selected key |
| `Space` | Toggle key selection |
| `Ctrl+a` | Select all |
| `/` | Search keys |
| `>` | Enter command mode |
| `Ctrl+n` | Switch database |
| `F5` | Refresh |
| `Ctrl+e` | Export |
| `Ctrl+l` | Import |
| `Ctrl+t` | Switch connection |
| `Ctrl+b` | Disconnect |
| `Delete` | Delete selected keys |

## KeyContent Mode (Vim-style)

### Normal Mode (Default)
| Key | Function |
|-----|----------|
| `hjkl` | Navigate left/down/up/right |
| `w` / `e` / `b` | Word navigation |
| `0` / `$` | Start/end of line |
| `gg` / `G` | Start/end of file |
| `Ctrl+d` / `Ctrl+u` | Half page down/up |
| `i` | Enter Insert mode |
| `a` | Append after cursor |
| `A` | Append at end of line |
| `o` / `O` | New line below/above |
| `v` | Enter Visual mode |
| `x` | Delete character |
| `dd` | Delete line |
| `yy` | Copy line |
| `p` | Paste |
| `u` | Undo |
| `Ctrl+r` | Redo |
| `ciw` | Change word |
| `:w` | Save to Redis (Vim-style) |
| `:q` | Return to Dashboard |
| `:wq` / `:x` | Save and return |
| `:q!` | Force quit without saving |
| `Ctrl+s` | Save (compatibility) |

### Insert Mode
| Key | Function |
|-----|----------|
| Characters | Normal input |
| `Enter` | New line |
| `Backspace` | Delete previous character |
| `Esc` | Return to Normal mode |

### Visual Mode
| Key | Function |
|-----|----------|
| `hjkl` | Extend selection |
| `y` | Copy |
| `d` | Delete |
| `Esc` | Return to Normal mode |

## Command Mode
| Key | Function |
|-----|----------|
| Type command | Enter Redis command |
| `Enter` | Execute command |
| `Esc` | Exit command mode |
| `↑↓` | Scroll output |

## Common Workflows

### Edit JSON Value (Vim-style)
1. Dashboard: Select key → `Enter`
2. Normal mode: `i` to enter Insert
3. Insert mode: Edit content
4. `Esc` to return to Normal
5. `:w` to save (or `:wq` to save and return)
6. `:q` to return to Dashboard

### Quick Word Change
1. Normal mode: Move to word
2. `ciw` to change word
3. Type new content
4. `Esc` to return to Normal
5. `:wq` to save and return

### Copy and Paste
1. Normal mode: `v` to enter Visual
2. `hjkl` to select content
3. `y` to copy
4. Move to target position
5. `p` to paste

## Tips
- Not familiar with Vim? Use arrow keys and mouse
- Press `Esc` to always return to Normal mode or exit command mode
- Press `:` in Normal mode to enter Vim command mode
- Supports `:w` (save), `:q` (quit), `:wq` (save & quit), `:q!` (force quit)
- `Ctrl+s` is kept for compatibility
- Use `:q` or `:q!` to return to Dashboard
