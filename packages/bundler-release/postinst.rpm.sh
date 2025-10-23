#!/bin/bash
# RPM postinst script for kodegen
# Runs kodegen_install automatically after package installation

set -e

# Run kodegen_install in non-interactive mode
# --from-platform rpm: Tells installer binaries are in /usr/bin
# --no-interaction: Headless mode for package manager
if [ -x /usr/bin/kodegen_install ]; then
    echo "Running kodegen installer..."
    /usr/bin/kodegen_install --from-platform rpm --no-interaction
    echo "Kodegen installation complete"
else
    echo "Warning: kodegen_install not found at /usr/bin/kodegen_install"
    echo "Manual installation may be required"
fi

exit 0
