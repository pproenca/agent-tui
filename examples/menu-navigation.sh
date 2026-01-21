#!/bin/bash
# Menu navigation workflow example
# Demonstrates interaction with dialog-based menus

set -e

echo "=== Menu Navigation Workflow ==="
echo

# Check if dialog is available
if ! command -v dialog &> /dev/null; then
    echo "Note: 'dialog' not installed. Using a simpler menu approach."

    # Create a simple menu script
    TEMP_SCRIPT=$(mktemp)
    cat > "$TEMP_SCRIPT" << 'EOF'
#!/bin/bash
while true; do
    echo "====== Main Menu ======"
    echo "1) Option A - Do something"
    echo "2) Option B - Do something else"
    echo "3) Option C - Another action"
    echo "4) Quit"
    echo "======================="
    read -p "Select option [1-4]: " choice
    case $choice in
        1) echo "You selected Option A" ;;
        2) echo "You selected Option B" ;;
        3) echo "You selected Option C" ;;
        4) echo "Goodbye!"; exit 0 ;;
        *) echo "Invalid option" ;;
    esac
    echo
done
EOF
    chmod +x "$TEMP_SCRIPT"

    echo "1. Starting menu application..."
    agent-tui spawn "$TEMP_SCRIPT"

    echo "2. Waiting for menu..."
    agent-tui wait "Main Menu"

    echo "3. Taking snapshot..."
    agent-tui snapshot -i

    echo "4. Selecting option 2..."
    agent-tui type "2"
    agent-tui press Enter

    echo "5. Verifying selection..."
    agent-tui wait "Option B"
    agent-tui snapshot

    echo "6. Selecting quit..."
    agent-tui type "4"
    agent-tui press Enter

    echo "7. Verifying quit..."
    agent-tui wait "Goodbye"

    echo "8. Cleaning up..."
    agent-tui kill
    rm -f "$TEMP_SCRIPT"
else
    echo "1. Starting dialog menu..."
    # Using dialog for a nice TUI menu
    agent-tui spawn bash -c 'dialog --menu "Main Menu" 15 50 4 1 "Option A" 2 "Option B" 3 "Option C" 4 "Quit" 2>&1'

    echo "2. Waiting for menu to appear..."
    sleep 1
    agent-tui snapshot -i

    echo "3. Navigating menu..."
    agent-tui press ArrowDown
    agent-tui press Enter

    echo "4. Final state..."
    agent-tui snapshot

    echo "5. Cleaning up..."
    agent-tui kill
fi

echo
echo "=== Workflow Complete ==="
