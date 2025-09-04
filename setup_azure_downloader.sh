#!/bin/bash
"""
Setup Azure Blob Downloader
Configures the environment and cron job for automatic downloads
"""

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_SCRIPT="$SCRIPT_DIR/azure_blob_downloader.py"
VENV_DIR="$SCRIPT_DIR/venv_azure"
LOG_FILE="$SCRIPT_DIR/setup.log"

echo "Setting up Azure Blob Downloader..." | tee "$LOG_FILE"

# Check if Python 3 is available
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is required but not installed" | tee -a "$LOG_FILE"
    exit 1
fi

echo "Creating virtual environment..." | tee -a "$LOG_FILE"
# Create virtual environment
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
fi

# Activate virtual environment
source "$VENV_DIR/bin/activate"

echo "Installing Python dependencies..." | tee -a "$LOG_FILE"
# Install dependencies
pip install --upgrade pip
pip install -r "$SCRIPT_DIR/requirements_azure.txt"

# Make the Python script executable
chmod +x "$PYTHON_SCRIPT"

echo "Creating configuration file..." | tee -a "$LOG_FILE"
# Create sample configuration if it doesn't exist
if [ ! -f "$SCRIPT_DIR/azure_config.json" ]; then
    python3 "$PYTHON_SCRIPT" --create-config
    echo "Please edit azure_config.json with your Azure credentials before running the script" | tee -a "$LOG_FILE"
fi

# Create wrapper script for cron
WRAPPER_SCRIPT="$SCRIPT_DIR/run_azure_download.sh"
cat > "$WRAPPER_SCRIPT" << EOF
#!/bin/bash
# Azure Blob Downloader Cron Wrapper
cd "$SCRIPT_DIR"
source "$VENV_DIR/bin/activate"
python3 "$PYTHON_SCRIPT"
EOF

chmod +x "$WRAPPER_SCRIPT"

echo "Setting up cron job..." | tee -a "$LOG_FILE"
# Create cron job (runs twice daily at 6:00 AM and 6:00 PM)
CRON_JOB="0 6,18 * * * $WRAPPER_SCRIPT >> $SCRIPT_DIR/logs/cron.log 2>&1"

# Check if cron job already exists
if crontab -l 2>/dev/null | grep -q "$WRAPPER_SCRIPT"; then
    echo "Cron job already exists" | tee -a "$LOG_FILE"
else
    # Add cron job
    (crontab -l 2>/dev/null; echo "$CRON_JOB") | crontab -
    echo "Cron job added: runs daily at 6:00 AM and 6:00 PM" | tee -a "$LOG_FILE"
fi

echo "Creating log directories..." | tee -a "$LOG_FILE"
# Create necessary directories
mkdir -p "$SCRIPT_DIR/logs"
mkdir -p "$SCRIPT_DIR/downloads"

echo "Setup completed!" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"
echo "Next steps:" | tee -a "$LOG_FILE"
echo "1. Edit azure_config.json with your Azure Storage credentials" | tee -a "$LOG_FILE"
echo "2. Test the script: $WRAPPER_SCRIPT" | tee -a "$LOG_FILE"
echo "3. Check logs in: $SCRIPT_DIR/logs/" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"
echo "The script will run automatically twice daily at 6:00 AM and 6:00 PM" | tee -a "$LOG_FILE"

# Show current cron jobs
echo "Current cron jobs:" | tee -a "$LOG_FILE"
crontab -l | tee -a "$LOG_FILE"
