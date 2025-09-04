#!/usr/bin/env python3
"""
Azure Blob Container Downloader
Downloads all files from a specified Azure blob container.
Runs twice daily via cron job.
"""

import os
import sys
import logging
from datetime import datetime
from pathlib import Path
from azure.storage.blob import BlobServiceClient
import argparse
import json

# Configuration
CONFIG_FILE = "azure_config.json"
LOG_DIR = "logs"
DOWNLOAD_DIR = "downloads"

def setup_logging():
    """Setup logging configuration"""
    # Create logs directory if it doesn't exist
    Path(LOG_DIR).mkdir(exist_ok=True)
    
    # Setup logging
    log_filename = f"{LOG_DIR}/azure_download_{datetime.now().strftime('%Y%m%d')}.log"
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler(log_filename),
            logging.StreamHandler(sys.stdout)
        ]
    )
    return logging.getLogger(__name__)

def load_config():
    """Load configuration from JSON file and environment variables"""
    try:
        with open(CONFIG_FILE, 'r') as f:
            config = json.load(f)
        
        # Override with environment variables if present
        if 'AZURE_CONNECTION_STRING' in os.environ:
            config['connection_string'] = os.environ['AZURE_CONNECTION_STRING']
        
        if 'AZURE_CONTAINER_NAME' in os.environ:
            config['container_name'] = os.environ['AZURE_CONTAINER_NAME']
        
        if 'AZURE_DOWNLOAD_DIR' in os.environ:
            config['download_directory'] = os.environ['AZURE_DOWNLOAD_DIR']
            
        if 'AZURE_MAX_FILE_SIZE_MB' in os.environ:
            config['max_file_size_mb'] = int(os.environ['AZURE_MAX_FILE_SIZE_MB'])
            
        if 'AZURE_OVERWRITE_EXISTING' in os.environ:
            config['overwrite_existing'] = os.environ['AZURE_OVERWRITE_EXISTING'].lower() in ('true', '1', 'yes')
        
        return config
    except FileNotFoundError:
        logger.error(f"Configuration file {CONFIG_FILE} not found. Please create it with Azure credentials.")
        return None
    except json.JSONDecodeError as e:
        logger.error(f"Error parsing configuration file: {e}")
        return None
    except ValueError as e:
        logger.error(f"Error parsing environment variable: {e}")
        return None

def create_sample_config():
    """Create a sample configuration file"""
    sample_config = {
        "connection_string": "DefaultEndpointsProtocol=https;AccountName=your_account;AccountKey=your_key;EndpointSuffix=core.windows.net",
        "container_name": "your-container-name",
        "download_directory": "./downloads",
        "overwrite_existing": False,
        "file_extensions_filter": [".bin", ".log", ".txt"],
        "max_file_size_mb": 100
    }
    
    with open(CONFIG_FILE, 'w') as f:
        json.dump(sample_config, f, indent=2)
    
    print(f"Sample configuration created at {CONFIG_FILE}")
    print("Please edit the file with your Azure credentials and settings.")

def download_blob_files(config, logger):
    """Download all files from Azure blob container"""
    try:
        # Create download directory
        download_dir = Path(config.get('download_directory', DOWNLOAD_DIR))
        download_dir.mkdir(exist_ok=True)
        
        # Initialize blob service client
        blob_service_client = BlobServiceClient.from_connection_string(config['connection_string'])
        container_name = config['container_name']
        
        logger.info(f"Connecting to container: {container_name}")
        
        # Get container client
        container_client = blob_service_client.get_container_client(container_name)
        
        # List all blobs in container
        blob_list = container_client.list_blobs()
        
        downloaded_count = 0
        skipped_count = 0
        error_count = 0
        
        for blob in blob_list:
            try:
                blob_name = blob.name
                local_path = download_dir / blob_name
                
                # Create subdirectories if needed
                local_path.parent.mkdir(parents=True, exist_ok=True)
                
                # Check file extension filter
                file_extensions = config.get('file_extensions_filter', [])
                if file_extensions and not any(blob_name.lower().endswith(ext.lower()) for ext in file_extensions):
                    logger.debug(f"Skipping {blob_name} - extension not in filter")
                    skipped_count += 1
                    continue
                
                # Check file size limit
                max_size_mb = config.get('max_file_size_mb', 100)
                if blob.size > max_size_mb * 1024 * 1024:
                    logger.warning(f"Skipping {blob_name} - size {blob.size / (1024*1024):.2f}MB exceeds limit of {max_size_mb}MB")
                    skipped_count += 1
                    continue
                
                # Check if file already exists
                if local_path.exists() and not config.get('overwrite_existing', False):
                    logger.debug(f"Skipping {blob_name} - file already exists")
                    skipped_count += 1
                    continue
                
                # Download the blob
                logger.info(f"Downloading {blob_name} ({blob.size / (1024*1024):.2f}MB)")
                
                blob_client = blob_service_client.get_blob_client(
                    container=container_name, 
                    blob=blob_name
                )
                
                with open(local_path, 'wb') as download_file:
                    download_file.write(blob_client.download_blob().readall())
                
                downloaded_count += 1
                logger.info(f"Successfully downloaded: {blob_name}")
                
            except Exception as e:
                logger.error(f"Error downloading {blob_name}: {e}")
                error_count += 1
                continue
        
        # Summary
        logger.info(f"Download completed - Downloaded: {downloaded_count}, Skipped: {skipped_count}, Errors: {error_count}")
        return downloaded_count, skipped_count, error_count
        
    except Exception as e:
        logger.error(f"Error accessing Azure blob container: {e}")
        return 0, 0, 1

def main():
    """Main function"""
    parser = argparse.ArgumentParser(description='Azure Blob Container Downloader')
    parser.add_argument('--create-config', action='store_true', 
                       help='Create sample configuration file')
    parser.add_argument('--config', default=CONFIG_FILE,
                       help='Configuration file path')
    
    args = parser.parse_args()
    
    # Create sample config if requested
    if args.create_config:
        create_sample_config()
        return
    
    # Setup logging
    global logger
    logger = setup_logging()
    
    logger.info("Starting Azure blob download process")
    
    # Load configuration
    config = load_config()
    if not config:
        logger.error("Failed to load configuration. Use --create-config to create a sample.")
        sys.exit(1)
    
    # Validate required config
    required_keys = ['connection_string', 'container_name']
    missing_keys = [key for key in required_keys if key not in config]
    if missing_keys:
        logger.error(f"Missing required configuration keys: {missing_keys}")
        sys.exit(1)
    
    # Download files
    downloaded, skipped, errors = download_blob_files(config, logger)
    
    if errors > 0:
        logger.warning(f"Process completed with {errors} errors")
        sys.exit(1)
    else:
        logger.info("Process completed successfully")

if __name__ == "__main__":
    main()
