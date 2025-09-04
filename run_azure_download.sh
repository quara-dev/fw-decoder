#!/bin/bash
# Azure Blob Downloader Cron Wrapper
cd "/home/quara/research/fw-decoder"
source "/home/quara/research/fw-decoder/venv_azure/bin/activate"
python3 "/home/quara/research/fw-decoder/azure_blob_downloader.py"
