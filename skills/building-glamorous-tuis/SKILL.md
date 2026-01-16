---
name: building-glamorous-tuis
description: >-
  Build stunning terminal user interfaces with the complete Charmbracelet ecosystem.
  TUI Building: Bubble Tea (framework), Bubbles (components), Lip Gloss (styling),
  Huh (forms), Glamour (markdown), Harmonica (animation), Wish (SSH apps), Log
  (logging), Gum (shell scripts). CLI Tools: VHS (terminal recording), Glow
  (markdown viewer), Mods (AI CLI), Freeze (code screenshots). Infrastructure:
  Soft Serve (Git server), Pop (email), Skate (key-value), Melt (SSH backup),
  Wishlist (SSH gateway). Development: teatest, x/term. Use when building Go CLI
  tools, terminal UIs, SSH apps, or when any CLI could benefit from glamour.
---

# Building Glamorous TUIs with Charmbracelet

## Why Charm Exists

**The Problem:** Most CLIs look like this:

```
$ mytool list
item1
item2
item3
$ mytool select
Enter item number: 2
Selected: item2
```

**The Charm Version:**

```
┌──────────────────────────────────┐
│  My Tool                    ⌘?   │
├──────────────────────────────────┤
│  ○ item1                         │
│  ● item2  ← selected             │
│  ○ item3                         │
│                                  │
│  ↑/↓: navigate  enter: select    │
└──────────────────────────────────┘
```

Same functionality. Completely different experience.

**When to automatically reach for Charm:**

| If you're doing this... | Use this |
|------------------------|----------|
| **Building TUIs** | |
| Any Go CLI with user interaction | Bubble Tea + Lip Gloss |
| Collecting user input | Huh (forms) |
| Showing lists/tables | Bubbles list/table |
| Displaying help/docs | Glamour + viewport |
| Progress/loading states | Bubbles spinner/progress |
| File/directory selection | Bubbles filepicker |
| Shell scripts needing UI | Gum CLI |
| Serving TUI over network | Wish SSH |
| Smooth animations | Harmonica springs |
| **CLI Tools** | |
| Recording terminal demos/GIFs | VHS |
| Viewing markdown in terminal | Glow (CLI) or Glamour (library) |
| AI assistance in terminal | Mods |
| Code screenshots for docs | Freeze |
| **Infrastructure** | |
| Self-hosted Git server | Soft Serve |
| Sending emails from scripts | Pop |
| Storing secrets/config | Skate |
| Backing up SSH keys | Melt |
| Multi-app SSH gateway | Wishlist |
| **Development** | |
| Testing TUI apps | x/exp/teatest |
| Terminal capability detection | x/term |

---

## THE EXACT PROMPTS

### Prompt 1: "Make My CLI Glamorous"

```
I have a Go CLI tool that currently uses fmt.Println and flag parsing.
Transform it into a polished TUI using Charmbracelet libraries:

1. Replace all fmt.Println output with Lip Gloss styled text
2. Replace any user prompts with Huh forms or Bubbles inputs
3. Add a proper help screen using Glamour for markdown rendering
4. Add keyboard navigation with clear visual feedback
5. Handle terminal resize gracefully
6. Add a loading spinner for any async operations
7. Use the alt screen for full-window mode

Preserve all existing functionality while dramatically improving UX.
Show me the complete transformed code.
```

### Prompt 2: "Build a TUI Dashboard"

```
Create a terminal dashboard using Charmbracelet that displays:
- A header with app name and status
- A sidebar with navigation (list component)
- A main content area (viewport for scrolling)
- A footer with keyboard hints (help component)

Requirements:
- Responsive to terminal resize
- Mouse support for clicking items
- Smooth transitions when switching views
- Proper focus management between panes
- Clean exit behavior (restore terminal state)

Use Bubble Tea for state, Bubbles for components, Lip Gloss for layout.
```

### Prompt 3: "Add Charm to Existing cobra/urfave CLI"

```
I have an existing CLI using [cobra/urfave/flag]. Add Charm polish:

1. Keep the existing command structure
2. Add interactive mode when run without args
3. Style all output with Lip Gloss
4. Add progress bars for long operations
5. Add confirmation prompts for destructive actions
6. Show errors in styled error boxes
7. Add --no-tui flag to disable for scripting

Show me how to integrate without breaking existing behavior.
```

---

## The 5-Minute TUI

**Get something running NOW.** Copy this, modify the items, ship it:

```go
package main

import (
    "fmt"
    "os"

    tea "github.com/charmbracelet/bubbletea"
    "github.com/charmbracelet/lipgloss"
)

var (
    selected = lipgloss.NewStyle().Foreground(lipgloss.Color("212")).Bold(true)
    normal   = lipgloss.NewStyle().Foreground(lipgloss.Color("252"))
    title    = lipgloss.NewStyle().Bold(true).Padding(0, 1).Background(lipgloss.Color("62"))
)

type model struct {
    items  []string
    cursor int
}

func (m model) Init() tea.Cmd { return nil }

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "q", "ctrl+c":
            return m, tea.Quit
        case "up", "k":
            if m.cursor > 0 {
                m.cursor--
            }
        case "down", "j":
            if m.cursor < len(m.items)-1 {
                m.cursor++
            }
        case "enter":
            fmt.Printf("\nYou chose: %s\n", m.items[m.cursor])
            return m, tea.Quit
        }
    }
    return m, nil
}

func (m model) View() string {
    s := title.Render("Select an item") + "\n\n"
    for i, item := range m.items {
        cursor := "  "
        style := normal
        if m.cursor == i {
            cursor = "▸ "
            style = selected
        }
        s += cursor + style.Render(item) + "\n"
    }
    s += "\n" + normal.Render("↑/↓: move • enter: select • q: quit")
    return s
}

func main() {
    m := model{items: []string{"Option A", "Option B", "Option C"}}
    if _, err := tea.NewProgram(m).Run(); err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }
}
```

**To run:** `go mod init example && go get github.com/charmbracelet/bubbletea github.com/charmbracelet/lipgloss && go run .`

---

## UI Pattern Recipes

### Pattern: Command Palette (Fuzzy Search)

```go
// Uses Bubbles list with filtering
items := []list.Item{
    item{title: "New File", key: "ctrl+n"},
    item{title: "Open File", key: "ctrl+o"},
    item{title: "Save", key: "ctrl+s"},
}
l := list.New(items, list.NewDefaultDelegate(), 40, 14)
l.Title = "Commands"
l.SetShowStatusBar(false)
l.SetFilteringEnabled(true)  // Built-in fuzzy search!
l.Styles.Title = titleStyle
```

### Pattern: Confirmation Dialog

```go
// With Huh (simplest)
var confirm bool
huh.NewConfirm().
    Title("Delete all files?").
    Description("This cannot be undone.").
    Affirmative("Yes, delete").
    Negative("Cancel").
    Value(&confirm).
    Run()

// Or styled with Lip Gloss
dialogStyle := lipgloss.NewStyle().
    Border(lipgloss.RoundedBorder()).
    BorderForeground(lipgloss.Color("205")).
    Padding(1, 2).
    Width(40)

dialog := dialogStyle.Render(
    titleStyle.Render("⚠️  Confirm Delete") + "\n\n" +
    "This will delete 42 files.\n\n" +
    "[Y]es  [N]o",
)
```

### Pattern: Split Pane Layout

```go
func (m model) View() string {
    // Calculate widths
    sideW := 30
    mainW := m.width - sideW - 3  // -3 for border

    // Style each pane
    sideStyle := lipgloss.NewStyle().
        Width(sideW).
        Height(m.height - 2).
        Border(lipgloss.RoundedBorder()).
        BorderForeground(lipgloss.Color("240"))

    mainStyle := lipgloss.NewStyle().
        Width(mainW).
        Height(m.height - 2).
        Border(lipgloss.RoundedBorder()).
        BorderForeground(lipgloss.Color("62"))

    // Render and join
    side := sideStyle.Render(m.sidebar.View())
    main := mainStyle.Render(m.content.View())

    return lipgloss.JoinHorizontal(lipgloss.Top, side, main)
}
```

### Pattern: Toast/Notification

```go
type model struct {
    toast       string
    toastTimer  int
    // ...
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case successMsg:
        m.toast = "✓ " + string(msg)
        m.toastTimer = 30  // frames
        return m, tick()
    case tickMsg:
        if m.toastTimer > 0 {
            m.toastTimer--
            return m, tick()
        }
        m.toast = ""
    }
    return m, nil
}

func (m model) View() string {
    view := m.mainContent()
    if m.toast != "" {
        toast := lipgloss.NewStyle().
            Background(lipgloss.Color("35")).
            Foreground(lipgloss.Color("255")).
            Padding(0, 2).
            Render(m.toast)
        // Position at top-right
        view = lipgloss.Place(m.width, m.height, lipgloss.Right, lipgloss.Top, toast)
    }
    return view
}
```

### Pattern: Progress with Details

```go
type model struct {
    progress progress.Model
    current  string
    done     int
    total    int
}

func (m model) View() string {
    pct := float64(m.done) / float64(m.total)

    return lipgloss.JoinVertical(lipgloss.Left,
        titleStyle.Render("Installing dependencies"),
        "",
        m.progress.ViewAs(pct),
        "",
        subtle.Render(fmt.Sprintf("(%d/%d) %s", m.done, m.total, m.current)),
    )
}
```

### Pattern: Tab Navigation

```go
type model struct {
    tabs      []string
    activeTab int
    // content for each tab...
}

func (m model) View() string {
    var renderedTabs []string

    for i, t := range m.tabs {
        style := inactiveTab
        if i == m.activeTab {
            style = activeTab
        }
        renderedTabs = append(renderedTabs, style.Render(t))
    }

    tabRow := lipgloss.JoinHorizontal(lipgloss.Top, renderedTabs...)
    content := m.tabContent[m.activeTab].View()

    return lipgloss.JoinVertical(lipgloss.Left, tabRow, content)
}

// Styles
var (
    activeTab = lipgloss.NewStyle().
        Bold(true).
        Border(lipgloss.RoundedBorder()).
        BorderForeground(lipgloss.Color("62")).
        Padding(0, 2)
    inactiveTab = lipgloss.NewStyle().
        Border(lipgloss.HiddenBorder()).
        Padding(0, 2)
)
```

### Pattern: Error Display

```go
func renderError(err error) string {
    errStyle := lipgloss.NewStyle().
        Border(lipgloss.RoundedBorder()).
        BorderForeground(lipgloss.Color("196")).
        Padding(1, 2).
        Width(60)

    titleStyle := lipgloss.NewStyle().
        Foreground(lipgloss.Color("196")).
        Bold(true)

    return errStyle.Render(
        titleStyle.Render("✗ Error") + "\n\n" +
        wordwrap.String(err.Error(), 56) + "\n\n" +
        subtle.Render("Press any key to continue"),
    )
}
```

**See [references/ui-recipes.md](references/ui-recipes.md) for 20+ complete patterns.**

---

## Core Architecture: The Elm Pattern

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Model    │───▸│   Update    │───▸│    View     │
│  (state)    │    │  (logic)    │    │  (render)   │
└─────────────┘    └─────────────┘    └─────────────┘
       ▲                  │
       │                  │
       └──────────────────┘
              Msg (events)
```

**Model:** All state in one struct. Width, height, cursor, data, error, loading...

**Update:** Pure function. `(model, msg) → (model, cmd)`. Never blocks. Never mutates.

**View:** Pure function. `model → string`. No side effects. Just render.

**Cmd:** Async work. Returns a Msg when done. HTTP calls, file I/O, timers...

```go
// The complete pattern
type model struct {
    width, height int      // Terminal size
    state         screen   // Current screen
    err           error    // Last error
    loading       bool     // Loading state
    // ... your data
}

func (m model) Init() tea.Cmd {
    return tea.Batch(
        loadInitialData,     // Async data fetch
        m.spinner.Tick,      // Start spinner
    )
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    // ALWAYS handle these first
    switch msg := msg.(type) {
    case tea.WindowSizeMsg:
        m.width, m.height = msg.Width, msg.Height
        // Resize all components here
    case tea.KeyMsg:
        if msg.String() == "ctrl+c" {
            return m, tea.Quit
        }
    case errMsg:
        m.err = msg.err
        m.loading = false
    }
    // Then delegate to current screen/components
    return m, nil
}

func (m model) View() string {
    if m.err != nil {
        return renderError(m.err)
    }
    if m.loading {
        return m.spinner.View() + " Loading..."
    }
    // Normal render
    return m.renderCurrentScreen()
}
```

---

## Library Quick Reference

| Library | Purpose | Install | Key Types |
|---------|---------|---------|-----------|
| **Bubble Tea** | TUI framework | `go get github.com/charmbracelet/bubbletea` | `tea.Model`, `tea.Cmd`, `tea.Msg` |
| **Bubbles** | Components | `go get github.com/charmbracelet/bubbles` | `list.Model`, `textinput.Model`, `viewport.Model`, `table.Model`, `spinner.Model`, `progress.Model` |
| **Lip Gloss** | Styling | `go get github.com/charmbracelet/lipgloss` | `lipgloss.Style`, `lipgloss.Color`, `lipgloss.Border` |
| **Huh** | Forms | `go get github.com/charmbracelet/huh` | `huh.Form`, `huh.Input`, `huh.Select`, `huh.Confirm` |
| **Glamour** | Markdown | `go get github.com/charmbracelet/glamour` | `glamour.Render()`, `glamour.NewTermRenderer()` |
| **Harmonica** | Animation | `go get github.com/charmbracelet/harmonica` | `harmonica.Spring`, `harmonica.FPS()` |
| **Wish** | SSH server | `go get github.com/charmbracelet/wish` | `wish.NewServer()`, middleware |
| **Log** | Logging | `go get github.com/charmbracelet/log` | `log.Info()`, `log.Error()` |

**v2 Track (bleeding edge):**
```bash
go get charm.land/bubbletea/v2@latest
go get charm.land/bubbles/v2@latest
go get charm.land/lipgloss/v2@latest
```

---

## Progressive Enhancement Path

### Level 1: Styled Output (10 min)

Replace `fmt.Println` with Lip Gloss:

```go
// Before
fmt.Println("Error: file not found")

// After
errStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("196")).Bold(true)
fmt.Println(errStyle.Render("Error: file not found"))
```

### Level 2: Interactive Prompts (30 min)

Replace `fmt.Scanf` with Huh:

```go
// Before
fmt.Print("Enter name: ")
fmt.Scanf("%s", &name)

// After
huh.NewInput().Title("Enter name").Value(&name).Run()
```

### Level 3: Full TUI (2-4 hours)

Convert to Bubble Tea with components:

```go
// Before: linear script
// After: event-driven TUI with multiple screens
```

### Level 4: Polish (ongoing)

Add animation, mouse support, themes, help system...

---

## Production Hardening

### Must-Have Checklist

```
□ Handle tea.WindowSizeMsg (responsive layout)
□ Handle ctrl+c gracefully (cleanup, restore terminal)
□ Log to file, not stdout (use tea.LogToFile)
□ Test with small terminals (80x24 minimum)
□ Test with no color (TERM=dumb, NO_COLOR=1)
□ Test with light AND dark backgrounds
□ Add --no-tui or --plain flag for scripting
□ Handle errors visually (don't just crash)
□ Show loading states for async operations
□ Include keyboard hints (help component)
```

### Optional but Impressive

```
□ Mouse support (WithMouseCellMotion)
□ Focus reporting (pause when backgrounded)
□ Alt screen (full-window mode)
□ Smooth animations (Harmonica springs)
□ Accessible mode (screen reader support)
□ Custom themes
□ Config file for preferences
□ VHS tape for README demo
```

---

## Debugging TUIs

TUI debugging is hard because you can't just print to stdout. Here's how:

### 1. File Logging

```go
// At program start
if os.Getenv("DEBUG") != "" {
    f, _ := tea.LogToFile("debug.log", "debug")
    defer f.Close()
}

// Then use log package
log.Printf("cursor moved to %d", m.cursor)
```

Run with: `DEBUG=1 go run . 2>&1 | tee debug.log`

Watch with: `tail -f debug.log` (in another terminal)

### 2. Debug View Mode

```go
func (m model) View() string {
    view := m.normalView()

    if m.debug {
        debug := fmt.Sprintf(
            "w=%d h=%d cursor=%d state=%v",
            m.width, m.height, m.cursor, m.state,
        )
        view += "\n" + lipgloss.NewStyle().
            Foreground(lipgloss.Color("240")).
            Render(debug)
    }
    return view
}
```

### 3. Message Tracing

```go
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    log.Printf("msg: %T %+v", msg, msg)  // Logs to file
    // ... rest of update
}
```

### 4. Panic Recovery

```go
func main() {
    defer func() {
        if r := recover(); r != nil {
            // Restore terminal before printing
            fmt.Fprintf(os.Stderr, "panic: %v\n%s", r, debug.Stack())
        }
    }()
    // ...
}
```

---

## Anti-Patterns

| Anti-Pattern | Why Bad | Fix |
|--------------|---------|-----|
| Blocking in Update | Freezes entire UI | Use commands for I/O |
| Ignoring WindowSizeMsg | Broken layout on resize | Always handle, resize components |
| Logging to stdout | Corrupts TUI display | Log to file |
| Hardcoded dimensions | Breaks on different terminals | Calculate from WindowSizeMsg |
| Mutating model directly | Unpredictable state | Return new model from Update |
| Deeply nested Views | Hard to maintain | Extract render functions |
| One giant Update switch | Unmaintainable | Delegate to screen/component handlers |
| Raw ANSI codes | Won't adapt to terminal | Use Lip Gloss |
| Manual prompt loops | Reinventing Huh poorly | Use Huh forms |

---

## When NOT to Use Charm

Charm adds complexity. Skip it when:

- **Output is piped:** `mytool | grep foo` — use plain text
- **No interaction needed:** Pure data transformation — just print
- **CI/CD scripts:** Headless environments — use flags/env vars
- **Very simple prompts:** One yes/no — maybe `fmt.Scanf` is fine
- **Non-Go project:** Use Gum for shell, or native tools

**Escape hatch pattern:**

```go
func main() {
    // Detect non-interactive
    if !term.IsTerminal(int(os.Stdin.Fd())) || os.Getenv("NO_TUI") != "" {
        runPlainMode()
        return
    }
    runTUI()
}
```

---

## Gum: Charm for Shell Scripts

When you can't use Go:

```bash
# Install
brew install gum

# Input
NAME=$(gum input --placeholder "Your name")

# Selection
COLOR=$(gum choose "red" "green" "blue")

# Multi-select
TOPPINGS=$(gum choose --no-limit "cheese" "pepperoni" "mushrooms")

# Confirmation
gum confirm "Deploy to production?" && ./deploy.sh

# Fuzzy filter from stdin
BRANCH=$(git branch | gum filter)

# Spinner
gum spin --spinner dot --title "Building..." -- make build

# Styled text
gum style --foreground 212 --border double "Hello World"

# Write multi-line (like textarea)
COMMIT_MSG=$(gum write --placeholder "Commit message...")

# Combine for git commit script
TYPE=$(gum choose "fix" "feat" "docs" "style" "refactor")
SCOPE=$(gum input --placeholder "scope")
MSG=$(gum input --placeholder "message")
gum confirm "Commit?" && git commit -m "$TYPE($SCOPE): $MSG"
```

---

## Charm CLI Tools

Standalone tools that enhance your terminal workflow.

### VHS: Terminal Recording & GIFs

Record terminal sessions as GIFs/videos for documentation:

```bash
# Install
brew install vhs

# Create a tape file
cat > demo.tape << 'EOF'
# VHS Tape - demo.tape
Output demo.gif
Set FontSize 14
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"

Type "echo 'Hello, World!'"
Sleep 500ms
Enter
Sleep 1s

Type "ls -la"
Enter
Sleep 2s
EOF

# Record
vhs demo.tape
```

**Key Commands:**
```tape
# Typing
Type "command"           # Type text
Type@100ms "slow"        # Type with delay between chars

# Actions
Enter                    # Press enter
Space                    # Press space
Backspace 5              # Delete 5 chars
Ctrl+C                   # Key combo
Tab                      # Tab key

# Timing
Sleep 1s                 # Wait 1 second
Sleep 500ms              # Wait 500ms

# Settings
Set FontSize 16
Set Width 1200
Set Height 600
Set Theme "Dracula"      # GitHub Dark, Catppuccin Mocha, etc.
Set TypingSpeed 50ms
Set Padding 20

# Output
Output demo.gif          # GIF output
Output demo.mp4          # Video output
Output demo.webm         # WebM output
```

**Pro tip:** Put `demo.tape` in your repo root, add to CI for auto-generated README GIFs.

### Glow: Terminal Markdown Viewer

Beautiful markdown rendering in the terminal:

```bash
# Install
brew install glow

# View a file
glow README.md

# View with pager (scrollable)
glow -p README.md

# Fetch and render URL
glow https://raw.githubusercontent.com/charmbracelet/glow/main/README.md

# Stash markdown for offline reading
glow stash README.md
glow stash list
glow stash show 1

# Set style
glow -s dark README.md    # dark, light, auto, notty, or custom JSON
```

**In Go (use Glamour library):**
```go
import "github.com/charmbracelet/glamour"

out, _ := glamour.Render("# Hello\n\nWorld!", "dark")
fmt.Print(out)

// Or with custom renderer
r, _ := glamour.NewTermRenderer(
    glamour.WithAutoStyle(),
    glamour.WithWordWrap(80),
)
out, _ := r.Render(markdown)
```

### Mods: AI on the Command Line

Pipe anything to AI, get answers:

```bash
# Install
brew install mods

# Basic usage
echo "Explain this error" | mods
cat error.log | mods "what's wrong?"

# Code review
git diff | mods "review this code for bugs"

# Generate code
mods "write a bash script to backup my dotfiles" > backup.sh

# With specific model
mods --model gpt-4 "complex question"

# Conversation mode
mods --continue "follow up question"

# Format output
mods --format "explain kubernetes" | glow  # Pipe to glow!

# Use with files
mods "summarize" < long-document.txt
cat *.go | mods "find potential bugs in this Go code"
```

**Configuration (~/.config/mods/mods.yml):**
```yaml
default-model: gpt-4
apis:
  openai:
    api-key-env: OPENAI_API_KEY
  anthropic:
    api-key-env: ANTHROPIC_API_KEY
```

**Power combo:** `git diff | mods "write commit message" | git commit -F -`

### Freeze: Code Screenshots

Generate beautiful code images for docs/social:

```bash
# Install
brew install freeze

# Basic usage
freeze main.go -o code.png

# With options
freeze main.go \
  --theme "catppuccin-mocha" \
  --font "JetBrains Mono" \
  --shadow \
  --padding 20 \
  --line-numbers \
  --window \
  -o beautiful-code.png

# From stdin
cat snippet.py | freeze --language python -o snippet.png

# Specific lines
freeze main.go --lines 10,20 -o function.png

# Configuration file
freeze --config freeze.json main.go
```

**freeze.json:**
```json
{
  "theme": "catppuccin-mocha",
  "font": { "family": "JetBrains Mono", "size": 14 },
  "shadow": { "blur": 20, "x": 0, "y": 10 },
  "padding": [20, 40, 20, 20],
  "line_numbers": true,
  "window": true
}
```

---

## Charm Infrastructure

Self-hosted services and utilities for your workflow.

### Soft Serve: Self-Hosted Git Server

A delicious Git server with TUI and SSH access:

```bash
# Install
brew install soft-serve

# Start server
soft serve

# Access via SSH (after setup)
ssh localhost -p 23231

# Clone repos
git clone ssh://localhost:23231/myrepo
```

**Server configuration (~/.config/soft-serve/config.yaml):**
```yaml
name: "My Soft Serve"
host: 0.0.0.0
port: 23231
initial_admin_keys:
  - "ssh-ed25519 AAAA... you@example.com"
```

**SSH TUI commands:**
```bash
ssh git.example.com          # Browse repos in TUI
ssh git.example.com repo create myrepo
ssh git.example.com repo delete myrepo
ssh git.example.com repo list
ssh git.example.com user list
```

### Pop: Send Emails from Terminal

Beautiful email sending:

```bash
# Install
brew install pop

# Send email
pop send \
  --from "me@example.com" \
  --to "you@example.com" \
  --subject "Hello" \
  --body "Message body"

# With attachment
pop send \
  --to "team@example.com" \
  --subject "Report" \
  --attach report.pdf \
  --body "See attached"

# From stdin (pipe markdown!)
cat update.md | pop send \
  --to "team@example.com" \
  --subject "Weekly Update"

# Interactive compose
pop
```

**Configuration (~/.config/pop/config.yaml):**
```yaml
from: me@example.com
smtp:
  host: smtp.gmail.com
  port: 587
  username: me@example.com
  password_env: SMTP_PASSWORD
```

### Skate: Personal Key-Value Store

Simple, encrypted key-value storage:

```bash
# Install
brew install skate

# Set values
skate set api_key "sk-1234567890"
skate set config.theme "dark"
skate set todo.1 "Buy milk"

# Get values
skate get api_key
skate get config.theme

# List all
skate list
skate list config.  # List with prefix

# Delete
skate delete todo.1

# Sync across machines (via Charm Cloud)
skate sync
```

**Use in scripts:**
```bash
# Store secrets for scripts
API_KEY=$(skate get api_key)
curl -H "Authorization: Bearer $API_KEY" https://api.example.com

# Configuration management
THEME=$(skate get config.theme || echo "light")
```

**In Go:**
```go
import "github.com/charmbracelet/skate"

db, _ := skate.Open("myapp")
defer db.Close()

db.Set("key", []byte("value"))
value, _ := db.Get("key")
```

### Melt: SSH Key Backup

Backup and restore SSH keys securely:

```bash
# Install
brew install melt

# Backup keys (encrypts with passphrase)
melt backup

# Creates: ~/.melt/backup.melt (encrypted)

# Restore on new machine
melt restore

# Backup to specific file
melt backup -o my-keys.melt

# Restore from specific file
melt restore -i my-keys.melt
```

**Workflow for new machine:**
```bash
# On old machine
melt backup -o keys.melt
# Transfer keys.melt securely to new machine

# On new machine
brew install melt
melt restore -i keys.melt
# Enter passphrase, keys restored!
```

### Wishlist: SSH App Directory

Serve multiple SSH apps on one port:

```bash
# Install
go install github.com/charmbracelet/wishlist@latest

# Configuration (~/.config/wishlist/config.yaml)
```

```yaml
# wishlist.yaml
listen: 0.0.0.0:22
endpoints:
  - name: git
    address: localhost:23231
  - name: chat
    address: localhost:2222
  - name: todos
    address: localhost:2223
```

```bash
# Run
wishlist serve

# Users connect and get a menu
ssh myserver.com
# Shows: [git] [chat] [todos] - pick one!
```

**Building Wishlist apps:**
```go
// Each app is a Wish (SSH) server
// Wishlist proxies to them based on user selection
```

---

## Testing & Terminal Utilities

Tools for testing and terminal detection.

### teatest: TUI Testing Framework

Headless testing for Bubble Tea apps:

```go
import (
    "testing"
    "time"
    tea "github.com/charmbracelet/bubbletea"
    "github.com/charmbracelet/x/exp/teatest"
)

func TestApp(t *testing.T) {
    m := NewModel()
    tm := teatest.NewTestModel(t, m)

    // Send key presses
    tm.Send(tea.KeyMsg{Type: tea.KeyDown})
    tm.Send(tea.KeyMsg{Type: tea.KeyEnter})

    // Type text
    tm.Type("hello world")

    // Wait for condition
    teatest.WaitFor(t, tm, func(bts []byte) bool {
        return strings.Contains(string(bts), "Expected output")
    }, teatest.WithDuration(time.Second))

    // Get final output
    out := tm.FinalOutput(t)
    if !strings.Contains(string(out), "success") {
        t.Fatal("expected success message")
    }

    // Quit
    tm.Send(tea.KeyMsg{Type: tea.KeyCtrlC})
    tm.WaitFinished(t, teatest.WithFinalTimeout(time.Second))
}
```

**Golden file testing:**
```go
func TestGolden(t *testing.T) {
    m := NewModel()
    tm := teatest.NewTestModel(t, m)

    tm.Send(tea.KeyMsg{Type: tea.KeyEnter})

    out := tm.FinalOutput(t)

    // Compare against saved "golden" output
    teatest.RequireEqualOutput(t, out)
    // First run: creates testdata/TestGolden.golden
    // Subsequent: compares against it
}

// Update golden files: go test -update
```

### x/term: Terminal Detection

Detect terminal capabilities:

```go
import "github.com/charmbracelet/x/term"

// Check if running in terminal
if term.IsTerminal(os.Stdin.Fd()) {
    runTUI()
} else {
    runPlainMode()
}

// Get terminal size
width, height, _ := term.GetSize(os.Stdout.Fd())

// Check color support
if term.HasDarkBackground() {
    useTheme("dark")
} else {
    useTheme("light")
}

// Environment detection
if os.Getenv("NO_COLOR") != "" || os.Getenv("TERM") == "dumb" {
    disableColors()
}
```

**Common patterns:**
```go
func main() {
    // Full detection
    isTTY := term.IsTerminal(os.Stdin.Fd())
    isPiped := !term.IsTerminal(os.Stdout.Fd())
    noColor := os.Getenv("NO_COLOR") != ""

    switch {
    case !isTTY:
        // stdin is piped, read from it
        runFilter()
    case isPiped:
        // stdout is piped, output plain text
        runPlainOutput()
    case noColor:
        // User requested no color
        runNoColor()
    default:
        // Full TUI mode
        runTUI()
    }
}
```

---

## Quick Start Commands

```bash
# TUI Libraries (Go)
go get github.com/charmbracelet/bubbletea@latest \
       github.com/charmbracelet/bubbles@latest \
       github.com/charmbracelet/lipgloss@latest \
       github.com/charmbracelet/huh@latest \
       github.com/charmbracelet/glamour@latest \
       github.com/charmbracelet/harmonica@latest \
       github.com/charmbracelet/wish@latest \
       github.com/charmbracelet/log@latest

# Testing utilities
go get github.com/charmbracelet/x/exp/teatest@latest \
       github.com/charmbracelet/x/term@latest

# CLI Tools (Homebrew)
brew install gum glow vhs freeze mods

# Infrastructure
brew install soft-serve pop skate melt

# Or install all at once
brew install charmbracelet/tap/gum \
             charmbracelet/tap/glow \
             charmbracelet/tap/vhs \
             charmbracelet/tap/freeze \
             charmbracelet/tap/mods \
             charmbracelet/tap/soft-serve \
             charmbracelet/tap/pop \
             charmbracelet/tap/skate \
             charmbracelet/tap/melt
```

---

## References

- **[Advanced Patterns](references/advanced-patterns.md)**: Complete app template, layout patterns, theming, testing, advanced architecture
- **[Component Catalog](references/component-catalog.md)**: All Bubbles components with detailed API examples

When in doubt: **make it beautiful**. The terminal deserves glamour.
