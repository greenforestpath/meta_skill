#!/bin/bash
# Process Triage - Interactive zombie/abandoned process killer
# Uses gum for UI and remembers past decisions
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════════════
# Configuration
# ══════════════════════════════════════════════════════════════════════════════
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/process_triage"
DECISIONS_FILE="$CONFIG_DIR/decisions.json"
PROTECTED_FILE="$CONFIG_DIR/protected.txt"
LOG_FILE="$CONFIG_DIR/triage.log"

# Ensure config directory exists
mkdir -p "$CONFIG_DIR"
touch "$DECISIONS_FILE" "$PROTECTED_FILE" "$LOG_FILE" 2>/dev/null || true

# Initialize decisions file if empty
[[ ! -s "$DECISIONS_FILE" ]] && echo '{}' > "$DECISIONS_FILE"

# ══════════════════════════════════════════════════════════════════════════════
# Styles
# ══════════════════════════════════════════════════════════════════════════════
HEADER_STYLE="bold magenta"
WARN_STYLE="bold yellow"
DANGER_STYLE="bold red"
SAFE_STYLE="bold green"
DIM_STYLE="faint"

# ══════════════════════════════════════════════════════════════════════════════
# Utility Functions
# ══════════════════════════════════════════════════════════════════════════════

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "$LOG_FILE"
}

format_time() {
    local seconds=$1
    if [[ $seconds -lt 60 ]]; then
        echo "${seconds}s"
    elif [[ $seconds -lt 3600 ]]; then
        echo "$((seconds / 60))m"
    elif [[ $seconds -lt 86400 ]]; then
        echo "$((seconds / 3600))h"
    else
        echo "$((seconds / 86400))d"
    fi
}

get_process_age_seconds() {
    local pid=$1
    local etime
    etime=$(ps -o etimes= -p "$pid" 2>/dev/null | tr -d ' ')
    echo "${etime:-0}"
}

get_memory_mb() {
    local pid=$1
    local rss
    rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ')
    echo "$((${rss:-0} / 1024))"
}

# ══════════════════════════════════════════════════════════════════════════════
# Decision Memory System
# ══════════════════════════════════════════════════════════════════════════════

# Get a decision key for a process (based on command pattern)
get_decision_key() {
    local cmd=$1
    # Normalize the command to create a pattern key
    # Remove PIDs, timestamps, unique identifiers
    echo "$cmd" | sed -E \
        -e 's/[0-9]{4,}//g' \
        -e 's/--port[= ][0-9]+/--port PORT/g' \
        -e 's/:[0-9]+/:PORT/g' \
        -e 's/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/UUID/g' \
        -e 's|/tmp/[^ ]+|/tmp/TMPFILE|g' \
        -e 's/[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/IP/g' \
        | tr -s ' ' | head -c 200
}

# Save a decision
save_decision() {
    local key=$1
    local decision=$2  # "kill" or "spare"
    local escaped_key
    escaped_key=$(echo "$key" | sed 's/"/\\"/g')

    # Use jq if available, otherwise simple append
    if command -v jq &>/dev/null; then
        local tmp
        tmp=$(mktemp)
        jq --arg key "$escaped_key" --arg dec "$decision" \
            '.[$key] = $dec' "$DECISIONS_FILE" > "$tmp" && mv "$tmp" "$DECISIONS_FILE"
    fi
    log "Saved decision: $decision for pattern: $key"
}

# Get a past decision
get_past_decision() {
    local key=$1
    local escaped_key
    escaped_key=$(echo "$key" | sed 's/"/\\"/g')

    if command -v jq &>/dev/null && [[ -s "$DECISIONS_FILE" ]]; then
        jq -r --arg key "$escaped_key" '.[$key] // "unknown"' "$DECISIONS_FILE" 2>/dev/null || echo "unknown"
    else
        echo "unknown"
    fi
}

# ══════════════════════════════════════════════════════════════════════════════
# Process Scoring System
# ══════════════════════════════════════════════════════════════════════════════

# Score a process (higher = more likely to be zombie/abandoned)
# Returns: score,recommendation,reason
score_process() {
    local pid=$1
    local cmd=$2
    local ppid=$3
    local age_seconds=$4
    local mem_mb=$5

    local score=0
    local reasons=()

    # Age-based scoring
    local age_hours=$((age_seconds / 3600))
    if [[ $age_hours -gt 168 ]]; then  # > 1 week
        score=$((score + 50))
        reasons+=("running >1 week")
    elif [[ $age_hours -gt 48 ]]; then  # > 2 days
        score=$((score + 30))
        reasons+=("running >2 days")
    elif [[ $age_hours -gt 24 ]]; then  # > 1 day
        score=$((score + 20))
        reasons+=("running >1 day")
    fi

    # Orphan detection (PPID=1)
    if [[ "$ppid" == "1" ]]; then
        score=$((score + 25))
        reasons+=("orphaned (PPID=1)")
    fi

    # Test process patterns
    if [[ "$cmd" =~ (bun\ test|jest|pytest|cargo\ test|npm\ test|vitest) ]]; then
        if [[ $age_seconds -gt 3600 ]]; then  # Tests > 1 hour
            score=$((score + 40))
            reasons+=("stuck test process")
        fi
    fi

    # Dev server patterns (less aggressive - might be intentional)
    if [[ "$cmd" =~ (--hot|--watch|dev\ server|next\ dev|vite) ]]; then
        if [[ $age_hours -gt 48 ]]; then
            score=$((score + 20))
            reasons+=("old dev server")
        fi
    fi

    # Claude/agent shell patterns
    if [[ "$cmd" =~ (claude|codex|gemini|anthropic) ]] && [[ "$cmd" =~ (bash|sh|shell) ]]; then
        if [[ $age_hours -gt 24 ]]; then
            score=$((score + 35))
            reasons+=("old agent shell")
        fi
    fi

    # High memory with long age
    if [[ $mem_mb -gt 1000 ]] && [[ $age_hours -gt 24 ]]; then
        score=$((score + 15))
        reasons+=("high memory (${mem_mb}MB)")
    fi

    # Protected patterns (reduce score)
    if [[ "$cmd" =~ (systemd|dbus|pulseaudio|pipewire|sshd|cron|docker) ]]; then
        score=$((score - 100))
        reasons+=("system service")
    fi

    # Check past decisions
    local key
    key=$(get_decision_key "$cmd")
    local past
    past=$(get_past_decision "$key")
    if [[ "$past" == "kill" ]]; then
        score=$((score + 20))
        reasons+=("killed similar before")
    elif [[ "$past" == "spare" ]]; then
        score=$((score - 30))
        reasons+=("spared similar before")
    fi

    # Determine recommendation
    local recommendation
    if [[ $score -ge 50 ]]; then
        recommendation="KILL"
    elif [[ $score -ge 20 ]]; then
        recommendation="REVIEW"
    else
        recommendation="SPARE"
    fi

    local reason_str
    reason_str=$(IFS=", "; echo "${reasons[*]}")
    echo "$score|$recommendation|$reason_str"
}

# ══════════════════════════════════════════════════════════════════════════════
# Process Collection
# ══════════════════════════════════════════════════════════════════════════════

collect_candidates() {
    local min_age_hours=${1:-1}  # Default: processes older than 1 hour
    local min_age_seconds=$((min_age_hours * 3600))

    local candidates=()

    # Get all user processes
    while IFS= read -r line; do
        local pid ppid cmd
        pid=$(echo "$line" | awk '{print $1}')
        ppid=$(echo "$line" | awk '{print $2}')
        cmd=$(echo "$line" | cut -d' ' -f3-)

        # Skip if can't get info
        [[ -z "$pid" ]] && continue
        [[ "$pid" == "PID" ]] && continue

        local age_seconds
        age_seconds=$(get_process_age_seconds "$pid")
        [[ $age_seconds -lt $min_age_seconds ]] && continue

        local mem_mb
        mem_mb=$(get_memory_mb "$pid")

        local score_result
        score_result=$(score_process "$pid" "$cmd" "$ppid" "$age_seconds" "$mem_mb")
        local score recommendation reason
        score=$(echo "$score_result" | cut -d'|' -f1)
        recommendation=$(echo "$score_result" | cut -d'|' -f2)
        reason=$(echo "$score_result" | cut -d'|' -f3)

        # Only include if score > 0 (potential candidates)
        if [[ $score -gt 0 ]]; then
            local age_fmt
            age_fmt=$(format_time "$age_seconds")
            candidates+=("$score|$pid|$recommendation|$age_fmt|${mem_mb}MB|$reason|$cmd")
        fi
    done < <(ps -eo pid,ppid,args --no-headers -u "$(whoami)" 2>/dev/null)

    # Sort by score (descending)
    printf '%s\n' "${candidates[@]}" | sort -t'|' -k1 -nr
}

# ══════════════════════════════════════════════════════════════════════════════
# Interactive UI
# ══════════════════════════════════════════════════════════════════════════════

show_header() {
    gum style \
        --border double \
        --border-foreground 212 \
        --padding "1 2" \
        --margin "1" \
        "$(gum style --foreground 212 --bold 'Process Triage')" \
        "$(gum style --foreground 245 'Interactive zombie/abandoned process killer')"
}

show_system_status() {
    local load cpu_count mem_used mem_total
    load=$(cat /proc/loadavg | cut -d' ' -f1-3)
    cpu_count=$(nproc)
    mem_used=$(free -h | awk '/^Mem:/ {print $3}')
    mem_total=$(free -h | awk '/^Mem:/ {print $2}')

    gum style \
        --foreground 245 \
        --margin "0 2" \
        "Load: $load (${cpu_count} cores) | Memory: $mem_used / $mem_total"
}

format_candidate_line() {
    local score=$1
    local pid=$2
    local rec=$3
    local age=$4
    local mem=$5
    local reason=$6
    local cmd=$7

    # Truncate command for display
    local cmd_short
    cmd_short=$(echo "$cmd" | head -c 60)
    [[ ${#cmd} -gt 60 ]] && cmd_short="${cmd_short}..."

    # Color based on recommendation
    local rec_color
    case $rec in
        KILL)   rec_color="red" ;;
        REVIEW) rec_color="yellow" ;;
        SPARE)  rec_color="green" ;;
    esac

    printf "%-6s %-7s %-6s %-8s %-6s │ %s\n" \
        "[$rec]" "PID:$pid" "$age" "$mem" "($score)" "$cmd_short"
}

run_interactive() {
    show_header
    show_system_status
    echo

    gum spin --spinner dot --title "Scanning processes..." -- sleep 0.5

    # Collect candidates
    local candidates
    mapfile -t candidates < <(collect_candidates 1)

    if [[ ${#candidates[@]} -eq 0 ]]; then
        gum style --foreground 10 --bold "No suspicious processes found!"
        exit 0
    fi

    gum style --foreground 212 --bold "Found ${#candidates[@]} candidate(s) for review:"
    echo

    # Build selection list with pre-populated choices
    local items=()
    local preselected=()
    local idx=0

    for candidate in "${candidates[@]}"; do
        IFS='|' read -r score pid rec age mem reason cmd <<< "$candidate"
        local line
        line=$(format_candidate_line "$score" "$pid" "$rec" "$age" "$mem" "$reason" "$cmd")
        items+=("$line")

        # Pre-select items recommended for killing
        if [[ "$rec" == "KILL" ]]; then
            preselected+=("$line")
        fi
        ((idx++))
    done

    # Show legend
    gum style --foreground 245 --margin "0 2" \
        "[KILL]=recommended  [REVIEW]=check  [SPARE]=probably safe"
    echo

    # Interactive selection
    local selected
    if [[ ${#preselected[@]} -gt 0 ]]; then
        selected=$(printf '%s\n' "${items[@]}" | gum choose --no-limit --height 20 \
            --header "Select processes to KILL (pre-selected are recommended):" \
            --selected "${preselected[@]}" 2>/dev/null || true)
    else
        selected=$(printf '%s\n' "${items[@]}" | gum choose --no-limit --height 20 \
            --header "Select processes to KILL:" 2>/dev/null || true)
    fi

    if [[ -z "$selected" ]]; then
        gum style --foreground 11 "No processes selected. Exiting."
        exit 0
    fi

    # Count selected
    local count
    count=$(echo "$selected" | wc -l)

    echo
    gum style --foreground 214 --bold "Selected $count process(es) to kill:"
    echo "$selected"
    echo

    # Confirm
    if ! gum confirm "Proceed with killing these $count process(es)?"; then
        gum style --foreground 11 "Aborted by user."
        exit 0
    fi

    # Extract PIDs and kill
    echo
    local killed=0
    local failed=0

    while IFS= read -r line; do
        # Extract PID from the line (format: [REC] PID:XXXX ...)
        local pid
        pid=$(echo "$line" | grep -oP 'PID:\K[0-9]+')

        if [[ -n "$pid" ]]; then
            # Find the original command for this PID
            local orig_cmd
            for candidate in "${candidates[@]}"; do
                if [[ "$candidate" == *"|$pid|"* ]]; then
                    orig_cmd=$(echo "$candidate" | cut -d'|' -f7)
                    break
                fi
            done

            gum spin --spinner line --title "Killing PID $pid..." -- kill "$pid" 2>/dev/null && {
                gum style --foreground 10 "  Killed PID $pid"
                ((killed++))

                # Save decision
                local key
                key=$(get_decision_key "$orig_cmd")
                save_decision "$key" "kill"
            } || {
                # Try SIGKILL
                gum spin --spinner line --title "Force killing PID $pid..." -- kill -9 "$pid" 2>/dev/null && {
                    gum style --foreground 11 "  Force killed PID $pid"
                    ((killed++))

                    local key
                    key=$(get_decision_key "$orig_cmd")
                    save_decision "$key" "kill"
                } || {
                    gum style --foreground 9 "  Failed to kill PID $pid"
                    ((failed++))
                }
            }
        fi
    done <<< "$selected"

    # Save "spare" decisions for unselected items
    for candidate in "${candidates[@]}"; do
        IFS='|' read -r score pid rec age mem reason cmd <<< "$candidate"
        local was_selected=false
        while IFS= read -r sel_line; do
            if [[ "$sel_line" == *"PID:$pid"* ]]; then
                was_selected=true
                break
            fi
        done <<< "$selected"

        if [[ "$was_selected" == "false" ]]; then
            local key
            key=$(get_decision_key "$cmd")
            save_decision "$key" "spare"
        fi
    done

    echo
    gum style --foreground 10 --bold "Done! Killed: $killed, Failed: $failed"

    # Show new system status
    echo
    show_system_status

    log "Triage complete: killed=$killed, failed=$failed"
}

# ══════════════════════════════════════════════════════════════════════════════
# Commands
# ══════════════════════════════════════════════════════════════════════════════

show_help() {
    cat << 'EOF'
Process Triage - Interactive zombie/abandoned process killer

USAGE:
    process_triage.sh [COMMAND] [OPTIONS]

COMMANDS:
    run         Interactive mode (default)
    scan        Scan and show candidates without killing
    clear       Clear decision history
    history     Show past decisions
    help        Show this help

OPTIONS:
    --min-age HOURS    Minimum process age to consider (default: 1)
    --dry-run          Show what would be done without killing

ENVIRONMENT:
    PROCESS_TRIAGE_CONFIG    Config directory (default: ~/.config/process_triage)

EXAMPLES:
    ./process_triage.sh                  # Interactive mode
    ./process_triage.sh scan             # Just scan, don't kill
    ./process_triage.sh --min-age 24     # Only processes older than 24h
    ./process_triage.sh clear            # Clear learned decisions
EOF
}

scan_only() {
    show_header
    show_system_status
    echo

    gum style --foreground 212 --bold "Scanning for candidate processes..."
    echo

    local candidates
    mapfile -t candidates < <(collect_candidates 1)

    if [[ ${#candidates[@]} -eq 0 ]]; then
        gum style --foreground 10 "No suspicious processes found!"
        exit 0
    fi

    printf "%-6s %-7s %-6s %-8s %-6s │ %s\n" \
        "REC" "PID" "AGE" "MEM" "SCORE" "COMMAND"
    echo "────────────────────────────────────────────────────────────────────────"

    for candidate in "${candidates[@]}"; do
        IFS='|' read -r score pid rec age mem reason cmd <<< "$candidate"
        format_candidate_line "$score" "$pid" "$rec" "$age" "$mem" "$reason" "$cmd"
    done

    echo
    gum style --foreground 245 "Run without arguments for interactive mode"
}

show_history() {
    if [[ ! -s "$DECISIONS_FILE" ]] || [[ "$(cat "$DECISIONS_FILE")" == "{}" ]]; then
        gum style --foreground 11 "No decision history yet."
        exit 0
    fi

    gum style --foreground 212 --bold "Past Decisions:"
    echo

    if command -v jq &>/dev/null; then
        jq -r 'to_entries[] | "\(.value | if . == "kill" then "KILL " else "SPARE" end)  \(.key)"' \
            "$DECISIONS_FILE" | sort | head -30
    else
        cat "$DECISIONS_FILE"
    fi
}

clear_history() {
    if gum confirm "Clear all decision history?"; then
        echo '{}' > "$DECISIONS_FILE"
        gum style --foreground 10 "Decision history cleared."
        log "Decision history cleared by user"
    else
        gum style --foreground 11 "Cancelled."
    fi
}

# ══════════════════════════════════════════════════════════════════════════════
# Dependencies
# ══════════════════════════════════════════════════════════════════════════════

ensure_gum() {
    if command -v gum &>/dev/null; then
        return 0
    fi

    echo "gum not found. Installing..."

    # Detect package manager and install
    if command -v apt-get &>/dev/null; then
        # Debian/Ubuntu
        echo "Installing gum via apt..."
        sudo mkdir -p /etc/apt/keyrings
        curl -fsSL https://repo.charm.sh/apt/gpg.key | sudo gpg --dearmor -o /etc/apt/keyrings/charm.gpg
        echo "deb [signed-by=/etc/apt/keyrings/charm.gpg] https://repo.charm.sh/apt/ * *" | sudo tee /etc/apt/sources.list.d/charm.list
        sudo apt-get update && sudo apt-get install -y gum
    elif command -v dnf &>/dev/null; then
        # Fedora/RHEL
        echo "Installing gum via dnf..."
        echo '[charm]
name=Charm
baseurl=https://repo.charm.sh/yum/
enabled=1
gpgcheck=1
gpgkey=https://repo.charm.sh/yum/gpg.key' | sudo tee /etc/yum.repos.d/charm.repo
        sudo dnf install -y gum
    elif command -v pacman &>/dev/null; then
        # Arch Linux
        echo "Installing gum via pacman..."
        sudo pacman -S --noconfirm gum
    elif command -v brew &>/dev/null; then
        # macOS/Linuxbrew
        echo "Installing gum via brew..."
        brew install gum
    elif command -v nix-env &>/dev/null; then
        # NixOS
        echo "Installing gum via nix..."
        nix-env -iA nixpkgs.gum
    else
        # Fallback: download binary directly
        echo "No package manager detected. Installing gum binary directly..."
        local version="0.14.1"
        local arch
        arch=$(uname -m)
        case "$arch" in
            x86_64) arch="amd64" ;;
            aarch64|arm64) arch="arm64" ;;
            *) echo "Unsupported architecture: $arch"; exit 1 ;;
        esac

        local os
        os=$(uname -s | tr '[:upper:]' '[:lower:]')

        local url="https://github.com/charmbracelet/gum/releases/download/v${version}/gum_${version}_${os}_${arch}.tar.gz"
        local tmp_dir
        tmp_dir=$(mktemp -d)

        echo "Downloading from $url..."
        curl -fsSL "$url" | tar -xz -C "$tmp_dir"

        if [[ -f "$tmp_dir/gum" ]]; then
            sudo mv "$tmp_dir/gum" /usr/local/bin/gum
            sudo chmod +x /usr/local/bin/gum
            echo "gum installed to /usr/local/bin/gum"
        else
            echo "Failed to extract gum binary"
            rm -rf "$tmp_dir"
            exit 1
        fi
        rm -rf "$tmp_dir"
    fi

    # Verify installation
    if ! command -v gum &>/dev/null; then
        echo "Failed to install gum. Please install manually."
        exit 1
    fi
    echo "gum installed successfully!"
}

# ══════════════════════════════════════════════════════════════════════════════
# Main
# ══════════════════════════════════════════════════════════════════════════════

main() {
    # Ensure gum is installed
    ensure_gum

    local cmd="${1:-run}"

    case "$cmd" in
        run|"")
            run_interactive
            ;;
        scan)
            scan_only
            ;;
        clear)
            clear_history
            ;;
        history)
            show_history
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo "Unknown command: $cmd"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
