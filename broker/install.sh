#!/bin/bash
set -e

ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

# Map architecture names
case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    aarch64) ARCH="aarch64" ;;
    arm64)   ARCH="aarch64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

echo "Installing cloudcli-broker for ${OS}-${ARCH}..."

# TODO: Replace with actual release URL
# curl -L "https://github.com/your-org/cloudcli/releases/latest/download/cloudcli-broker-${OS}-${ARCH}" -o "${INSTALL_DIR}/cloudcli-broker"
# chmod +x "${INSTALL_DIR}/cloudcli-broker"

echo "cloudcli-broker installed to ${INSTALL_DIR}/cloudcli-broker"

# Create systemd user service (Linux only)
if [ "$OS" = "linux" ] && command -v systemctl &> /dev/null; then
    SYSTEMD_DIR="${HOME}/.config/systemd/user"
    mkdir -p "$SYSTEMD_DIR"

    cat > "${SYSTEMD_DIR}/cloudcli-broker.service" << EOF
[Unit]
Description=CloudCLI Broker
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/cloudcli-broker --port 9999
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

    systemctl --user daemon-reload
    systemctl --user enable cloudcli-broker
    systemctl --user start cloudcli-broker

    echo "systemd service created and started"
fi

echo "Installation complete!"
