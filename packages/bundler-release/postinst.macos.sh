#!/bin/bash
# macOS postinst script for kodegen
# Runs kodegen_install automatically after .app installation

set -e

# Determine installation location
APP_DIR="/Applications/Kodegen.app"
KODEGEN_INSTALL="${APP_DIR}/Contents/MacOS/kodegen_install"

# Run kodegen_install in non-interactive mode
# --from-platform macos: Tells installer binaries are in .app bundle
# --no-interaction: Headless mode for installer
if [ -x "${KODEGEN_INSTALL}" ]; then
    echo "Running kodegen installer..."
    "${KODEGEN_INSTALL}" --from-platform macos --no-interaction
    echo "Kodegen installation complete"
else
    echo "Warning: kodegen_install not found at ${KODEGEN_INSTALL}"
    echo "Manual installation may be required"
fi

exit 0
