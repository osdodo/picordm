# Quick Reference - Keybindings

## Global Shortcuts

| Key      | Function         |
| -------- | ---------------- |
| `Ctrl+q` | Exit application |
| `Ctrl+p` | Open settings    |

## Dashboard Mode

| Key      | Function                 |
| -------- | ------------------------ |
| `↑↓`     | Select key               |
| `Enter`  | Open selected key        |
| `Space`  | Toggle key selection     |
| `Ctrl+a` | Select all/Cancel        |
| `/`      | Search keys              |
| `>`      | Enter command mode       |
| `Ctrl+n` | Switch database          |
| `Ctrl+r` | Refresh key list         |
| `F5`     | Refresh server stats     |
| `Ctrl+e` | Export                   |
| `Ctrl+l` | Import                   |
| `Ctrl+t` | Switch connection        |
| `Ctrl+b` | Disconnect               |
| `Delete` | Delete selected keys     |

## KeyContent Mode (Vim-style)

### Normal Mode (Default)

| Key                 | Function                    |
| ------------------- | --------------------------- |
| `hjkl`              | Navigate left/down/up/right |
| `w` / `e` / `b`     | Word navigation             |
| `0` / `$`           | Start/end of line           |
| `gg` / `G`          | Start/end of file           |
| `Ctrl+d` / `Ctrl+u` | Half page down/up           |
| `i`                 | Enter Insert mode           |
| `a`                 | Append after cursor         |
| `A`                 | Append at end of line       |
| `o` / `O`           | New line below/above        |
| `v`                 | Enter Visual mode           |
| `x`                 | Delete character            |
| `dd`                | Delete line                 |
| `yy`                | Copy line                   |
| `p`                 | Paste                       |
| `u`                 | Undo                        |
| `Ctrl+r`            | Redo                        |
| `ciw`               | Change word                 |
| `:w`                | Save to Redis (Vim-style)   |
| `:q`                | Return to Dashboard         |
| `:wq`               | Save and return             |
| `:q!`               | Force quit without saving   |

### Insert Mode

| Key         | Function                  |
| ----------- | ------------------------- |
| Characters  | Normal input              |
| `Enter`     | New line                  |
| `Backspace` | Delete previous character |
| `Esc`       | Return to Normal mode     |

### Visual Mode

| Key    | Function              |
| ------ | --------------------- |
| `hjkl` | Extend selection      |
| `y`    | Copy                  |
| `d`    | Delete                |
| `Esc`  | Return to Normal mode |

## Command Mode

Command mode supports two focus states, use `Tab` key to switch between input and browsing:

### Input Focus (Default)

| Key          | Function                |
| ------------ | ----------------------- |
| Type command | Enter Redis command     |
| `Enter`      | Execute command         |
| `Tab`        | Switch to browse output |
| `Esc`        | Exit command mode       |

### Browse Focus (View Command Output)

| Key                 | Function                                  |
| ------------------- | ----------------------------------------- |
| `hjkl` / `↑↓←→`     | Navigate output                           |
| `v`                 | Enter Visual mode to select text          |
| `i`                 | Enter Insert mode (not available for readonly output) |
| `Ctrl+d` / `Ctrl+u` | Half page down/up                         |
| `gg` / `G`          | Jump to start/end                         |
| `Esc`               | Exit Visual/Insert mode, or exit Command Mode |
| `Tab`               | Switch back to input                      |

**Tips**: 
- When there's command output, press `Tab` to switch to browse mode and use Vim-style keys (hjkl) to navigate
- In browse mode, press `v` to enter Visual mode for text selection, then `y` to copy
- In Visual or Insert mode, the first `Esc` exits that mode back to Normal mode, press `Esc` again to exit Command Mode
- Press `Tab` anytime to return to input mode for entering more commands

## Quick Connection Switch

Press `Ctrl+t` from Dashboard to open the Quick Connection Switch dialog:

### Navigation Mode (Default)

| Key      | Function                    |
| -------- | --------------------------- |
| `↑↓`     | Navigate connections        |
| `j`/`k`  | Navigate connections        |
| `1-9`    | Quick select (first 9)      |
| `/`      | Enter search mode           |
| `Enter`  | Switch to selected          |
| `Esc`    | Close dialog                 |

### Search Mode

Press `/` to enter search mode:

| Key         | Function                    |
| ----------- | --------------------------- |
| Type text   | Filter connections          |
| `↑↓`        | Navigate filtered results   |
| `Backspace` | Clear search text           |
| `Enter`     | Switch to selected          |
| `Esc`       | Exit search mode            |

**Tips**:
- Press `/` to activate search mode before typing
- Search matches connection name and host
- First matching result is auto-selected
- Press `Esc` in search mode to return to navigation mode
- Press `Esc` again to close the dialog

## Settings Dialog

Press `Ctrl+p` to open the Settings dialog:

| Key                | Function                    |
| ------------------ | --------------------------- |
| `↑↓` / `j`/`k`     | Navigate settings           |
| `Tab`              | Navigate settings           |
| `←→` / `h`/`l`     | Change theme (Dark/Light)   |
| `Space` / `Enter`  | Toggle current setting      |
| `Esc`              | Close settings              |

**Tips**:
- Theme changes apply immediately
- Settings are automatically saved when closing the dialog
- Dark theme supports background transparency option

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
