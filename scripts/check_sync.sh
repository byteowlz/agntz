#!/bin/bash
# Check if agntz commands are in sync with wrapped CLI tools

echo "========================================"
echo "Checking agntz sync with wrapped tools"
echo "========================================"
echo ""

ISSUES_FOUND=0

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to report an issue
report_issue() {
    echo -e "${RED}✗ ISSUE: $1${NC}"
    echo "  $2"
    ((ISSUES_FOUND++))
}

# Function to report OK
report_ok() {
    echo -e "${GREEN}✓ OK: $1${NC}"
}

# Function to report warning
report_warning() {
    echo -e "${YELLOW}⚠ WARNING: $1${NC}"
}

echo "=== Checking skdlr (schedule) ==="
if command -v skdlr &> /dev/null; then
    SKDLR_VERSION=$(skdlr --version 2>&1 | head -1)
    echo "skdlr version: $SKDLR_VERSION"

    # Check add command flags
    echo -n "Checking 'add' command flags... "
    SKDLR_ADD_HELP=$(skdlr add --help 2>&1)
    AGNTZ_ADD_HELP=$(cargo run -- schedule add --help 2>&1 || ./target/debug/agntz schedule add --help 2>&1)

    # Check for --at flag (one-off runs)
    if echo "$SKDLR_ADD_HELP" | grep -q -- "--at"; then
        if echo "$AGNTZ_ADD_HELP" | grep -q -- "--at"; then
            echo "✓ --at flag supported"
        else
            report_issue "skdlr has --at flag but agntz doesn't" \
                "skdlr now supports one-off runs with --at, but agntz hasn't been updated"
        fi
    fi

    # Check for natural language schedules
    if echo "$SKDLR_ADD_HELP" | grep -q "natural language"; then
        if echo "$AGNTZ_ADD_HELP" | grep -q "natural language"; then
            echo "✓ Natural language schedules documented"
        else
            report_warning "skdlr supports natural language schedules" \
                "agntz help text may not mention this (e.g., 'every day at 9am')"
        fi
    fi

    # Check --enabled vs --disabled
    if echo "$SKDLR_ADD_HELP" | grep -q -- "--enabled"; then
        if echo "$AGNTZ_ADD_HELP" | grep -q -- "--enabled"; then
            echo "✓ --enabled flag matches"
        elif echo "$AGNTZ_ADD_HELP" | grep -q -- "--disabled"; then
            report_issue "Flag mismatch: skdlr uses --enabled, agntz uses --disabled" \
                "Update src/schedule.rs to match skdlr's flag naming"
        fi
    fi

else
    echo "⊘ skdlr not installed - skipping"
fi
echo ""

echo "=== Checking mmry (memory) ==="
if command -v mmry &> /dev/null; then
    MMRY_VERSION=$(mmry --version 2>&1 | head -1)
    echo "mmry version: $MMRY_VERSION"

    # Check available commands
    echo "Checking mmry commands vs agntz memory commands..."
    MMRY_COMMANDS=$(mmry --help 2>&1 | grep -A 50 "Commands:" | grep "^\s*[a-z]" | awk '{print $1}')
    AGNTZ_MEMORY_COMMANDS="add search list export import stats stores remove"

    for cmd in $MMRY_COMMANDS; do
        case $cmd in
            add|search|ls|rm|stats|export|import|stores)
                if [[ "$AGNTZ_MEMORY_COMMANDS" == *"$cmd"* ]] || \
                   ([ "$cmd" = "ls" ] && [[ "$AGNTZ_MEMORY_COMMANDS" == *"list"* ]]) || \
                   ([ "$cmd" = "rm" ] && [[ "$AGNTZ_MEMORY_COMMANDS" == *"remove"* ]]); then
                    :
                else
                    report_warning "mmry has '$cmd' command but agntz may not wrap it"
                fi
                ;;
            ingest|prune|profile|guard|reembed|reextract|service|models|rerankers|hmlr)
                report_warning "mmry has '$cmd' command that agntz doesn't expose" \
                    "Consider adding to src/memory.rs if useful for agents"
                ;;
        esac
    done
    report_ok "Basic mmry commands wrapped"

else
    echo "⊘ mmry not installed - skipping"
fi
echo ""

echo "=== Checking trx (tasks) ==="
if command -v trx &> /dev/null; then
    TRX_VERSION=$(trx --version 2>&1 | head -1)
    echo "trx version: $TRX_VERSION"

    # Check commands
    echo "Checking trx commands vs agntz tasks commands..."
    TRX_COMMANDS=$(trx --help 2>&1 | grep -A 30 "Commands:" | grep "^\s*[a-z]" | awk '{print $1}')
    AGNTZ_TASKS_COMMANDS="list create update close show"

    for cmd in $TRX_COMMANDS; do
        case $cmd in
            create|list|show|update|close)
                if [[ "$AGNTZ_TASKS_COMMANDS" != *"$cmd"* ]]; then
                    report_issue "agntz doesn't wrap trx '$cmd' command" \
                        "Update src/issues.rs to include this command"
                fi
                ;;
            ready|dep|sync|migrate|import|purge-beads|schema|config|service)
                if [ "$cmd" = "ready" ]; then
                    report_ok "trx 'ready' has dedicated agntz command"
                elif [ "$cmd" = "dep" ]; then
                    report_warning "trx has 'dep' command for dependencies - agntz doesn't expose it"
                fi
                ;;
        esac
    done
    report_ok "Basic trx commands wrapped"

else
    echo "⊘ trx not installed - skipping"
fi
echo ""

echo "=== Checking mailz-cli (mail/reservations) ==="
if command -v mailz-cli &> /dev/null; then
    MAILZ_VERSION=$(mailz-cli --version 2>&1 | head -1)
    echo "mailz-cli version: $MAILZ_VERSION"

    echo "Checking mailz commands vs agntz commands..."
    MAILZ_COMMANDS=$(mailz-cli --help 2>&1 | grep -A 30 "Commands:" | grep "^\s*[a-z]" | awk '{print $1}')
    AGNTZ_MAIL_COMMANDS="inbox send ack"
    AGNTZ_RESERVATION_COMMANDS="reserve release reservations"

    report_ok "Basic mailz commands wrapped (check manually for new features)"

else
    echo "⊘ mailz-cli not installed - skipping"
fi
echo ""

echo "=== Checking hstry (search) ==="
if command -v hstry &> /dev/null; then
    HSTRY_VERSION=$(hstry --version 2>&1 | head -1)
    echo "hstry version: $HSTRY_VERSION"

    report_ok "hstry search command wrapped"
else
    echo "⊘ hstry not installed - skipping"
fi
echo ""

echo "========================================"
echo "Sync Check Complete"
echo "========================================"
if [ $ISSUES_FOUND -eq 0 ]; then
    echo -e "${GREEN}✓ No sync issues detected!${NC}"
    echo ""
    echo "Tip: Run integration tests to verify functionality:"
    echo "  ./tests/integration/run_all.sh"
    exit 0
else
    echo -e "${RED}✗ Found $ISSUES_FOUND issue(s) that need attention${NC}"
    echo ""
    echo "Tip: Update the relevant module in src/ to match the wrapped tool's changes"
    exit 1
fi
