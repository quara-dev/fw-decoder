#!/usr/bin/env python3
"""
Azure Blob Container Downloader
Downloads all files from a specified Azure blob container.
Runs twice daily via cron job.
"""

import os
import sys
import logging
import shutil
import threading
import time
from datetime import datetime
from pathlib import Path
from azure.storage.blob import BlobServiceClient
import argparse
import json
from concurrent.futures import ThreadPoolExecutor, as_completed

# Configuration
CONFIG_FILE = "azure_config.json"
LOG_DIR = "logs"
DOWNLOAD_DIR = "downloads"

# Resource optimization constants
CHUNK_SIZE = 8 * 1024 * 1024  # 8MB chunks for streaming downloads
MAX_CONCURRENT_DOWNLOADS = 3   # Limit concurrent downloads
DOWNLOAD_TIMEOUT = 600         # 10 minutes timeout per file
PROGRESS_REPORT_INTERVAL = 50  # Report progress every 50MB

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

def clear_download_directory(download_dir, logger):
    """Clear all files in the download directory"""
    try:
        if download_dir.exists():
            logger.info(f"Clearing existing files in {download_dir}")
            # Remove all files and subdirectories
            for item in download_dir.iterdir():
                if item.is_file():
                    item.unlink()
                    logger.debug(f"Deleted file: {item}")
                elif item.is_dir():
                    shutil.rmtree(item)
                    logger.debug(f"Deleted directory: {item}")
            logger.info("Download directory cleared successfully")
        else:
            logger.info(f"Download directory {download_dir} doesn't exist, will be created")
    except Exception as e:
        logger.error(f"Error clearing download directory: {e}")
        raise

def download_blob_with_progress(blob_client, local_path, blob_size, logger):
    """Download blob with progress reporting and memory efficiency"""
    try:
        downloaded_bytes = 0
        last_progress_report = 0
        
        with open(local_path, 'wb') as download_file:
            # Stream download in chunks to avoid loading entire file into memory
            stream = blob_client.download_blob()
            
            for chunk in stream.chunks():
                download_file.write(chunk)
                downloaded_bytes += len(chunk)
                
                # Report progress every PROGRESS_REPORT_INTERVAL MB
                if downloaded_bytes - last_progress_report >= PROGRESS_REPORT_INTERVAL * 1024 * 1024:
                    progress_pct = (downloaded_bytes / blob_size) * 100 if blob_size > 0 else 0
                    logger.info(f"Downloaded {downloaded_bytes / (1024*1024):.1f}MB of {blob_size / (1024*1024):.1f}MB ({progress_pct:.1f}%)")
                    last_progress_report = downloaded_bytes
        
        return True
        
    except Exception as e:
        logger.error(f"Error during streaming download: {e}")
        # Clean up partial file
        if local_path.exists():
            local_path.unlink()
        return False

def download_single_blob(blob_service_client, container_name, blob, config, logger):
    """Download a single blob with resource management"""
    try:
        blob_name = blob.name
        download_dir = Path(config.get('download_directory', DOWNLOAD_DIR))
        local_path = download_dir / blob_name
        
        # Create subdirectories if needed
        local_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Check file extension filter
        file_extensions = config.get('file_extensions_filter', [])
        if file_extensions and not any(blob_name.lower().endswith(ext.lower()) for ext in file_extensions):
            logger.debug(f"Skipping {blob_name} - extension not in filter")
            return 'skipped'
        
        # Check file size limit
        max_size_mb = config.get('max_file_size_mb', 100)
        if blob.size > max_size_mb * 1024 * 1024:
            logger.warning(f"Skipping {blob_name} - size {blob.size / (1024*1024):.2f}MB exceeds limit of {max_size_mb}MB")
            return 'skipped'
        
        # Download with timeout and progress reporting
        logger.info(f"Starting download: {blob_name} ({blob.size / (1024*1024):.2f}MB)")
        
        blob_client = blob_service_client.get_blob_client(
            container=container_name, 
            blob=blob_name
        )
        
        # Use streaming download for memory efficiency
        if download_blob_with_progress(blob_client, local_path, blob.size, logger):
            logger.info(f"Successfully downloaded: {blob_name}")
            return 'downloaded'
        else:
            return 'error'
            
    except Exception as e:
        logger.error(f"Error downloading {blob_name}: {e}")
        return 'error'
    """Download all files from Azure blob container"""
    try:
        # Create download directory
        download_dir = Path(config.get('download_directory', DOWNLOAD_DIR))
        
        # Clear existing files if requested
        if clear_existing:
            clear_download_directory(download_dir, logger)
            
        download_dir.mkdir(exist_ok=True)
        
        # Initialize blob service client
        blob_service_client = BlobServiceClient.from_connection_string(config['connection_string'])
        container_name = config['container_name']
        
        logger.info(f"Connecting to container: {container_name}")
        
        # Get container client
        container_client = blob_service_client.get_container_client(container_name)
        
        # List all blobs in container
        blob_list = list(container_client.list_blobs())  # Convert to list to get count
        logger.info(f"Found {len(blob_list)} files in container")
        
        downloaded_count = 0
        skipped_count = 0
        error_count = 0
        
        # Process downloads sequentially but with streaming to manage memory
        for blob in blob_list:
            try:
                # Skip if file already exists (only relevant if not clearing existing)
                if not clear_existing:
                    local_path = download_dir / blob.name
                    if local_path.exists() and not config.get('overwrite_existing', False):
                        logger.debug(f"Skipping {blob.name} - file already exists")
                        skipped_count += 1
                        continue
                
                result = download_single_blob(blob_service_client, container_name, blob, config, logger)
                
                if result == 'downloaded':
                    downloaded_count += 1
                elif result == 'skipped':
                    skipped_count += 1
                else:  # error
                    error_count += 1
                    
            except Exception as e:
                logger.error(f"Error processing {blob.name}: {e}")
                error_count += 1
                continue
        
        # Summary
        logger.info(f"Download completed - Downloaded: {downloaded_count}, Skipped: {skipped_count}, Errors: {error_count}")
        return downloaded_count, skipped_count, error_count
        
    except Exception as e:
        logger.error(f"Error accessing Azure blob container: {e}")
        return 0, 0, 1

def download_blob_files(config, logger, clear_existing=False):
    """Main function to orchestrate the blob download process."""
    try:
        # Initialize Azure client
        blob_service_client = BlobServiceClient.from_connection_string(config['connection_string'])
        container_client = blob_service_client.get_container_client(config['container_name'])
        
        # Clear existing files if requested
        if clear_existing:
            download_dir = config.get('download_directory', DOWNLOAD_DIR)
            if os.path.exists(download_dir):
                logger.info(f"Clearing existing directory: {download_dir}")
                import shutil
                shutil.rmtree(download_dir)
                os.makedirs(download_dir, exist_ok=True)
            else:
                os.makedirs(download_dir, exist_ok=True)
        
        # Get list of blobs
        logger.info("Fetching blob list from container...")
        blobs = []
        try:
            for blob in container_client.list_blobs():
                blobs.append(blob)
            logger.info(f"Found {len(blobs)} blobs to process")
        except Exception as e:
            logger.error(f"Failed to list blobs: {e}")
            return 0, 0, 1
        
        if not blobs:
            logger.info("No blobs found in container")
            return 0, 0, 0
        
        # Process downloads with resource management
        downloaded = 0
        skipped = 0
        errors = 0
        
        with ThreadPoolExecutor(max_workers=MAX_CONCURRENT_DOWNLOADS) as executor:
            # Submit download tasks
            future_to_blob = {}
            for blob in blobs:
                future = executor.submit(download_single_blob, 
                                       container_client, blob, config, logger)
                future_to_blob[future] = blob.name
            
            # Process completed downloads
            for future in as_completed(future_to_blob, timeout=DOWNLOAD_TIMEOUT):
                blob_name = future_to_blob[future]
                try:
                    result = future.result(timeout=30)  # 30 second timeout per result
                    if result == "downloaded":
                        downloaded += 1
                    elif result == "skipped":
                        skipped += 1
                    else:  # error
                        errors += 1
                        logger.error(f"Failed to download {blob_name}")
                except TimeoutError:
                    logger.error(f"Timeout downloading {blob_name}")
                    errors += 1
                except Exception as e:
                    logger.error(f"Exception downloading {blob_name}: {e}")
                    errors += 1
        
        logger.info(f"Download summary: {downloaded} downloaded, {skipped} skipped, {errors} errors")
        return downloaded, skipped, errors
        
    except Exception as e:
        logger.error(f"Critical error in download process: {e}")
        return 0, 0, 1

def main():
    """Main function"""
    parser = argparse.ArgumentParser(description='Azure Blob Container Downloader')
    parser.add_argument('--create-config', action='store_true', 
                       help='Create sample configuration file')
    parser.add_argument('--config', default=CONFIG_FILE,
                       help='Configuration file path')
    parser.add_argument('--clear-existing', action='store_true',
                       help='Clear all existing files before downloading new ones')
    
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
    downloaded, skipped, errors = download_blob_files(config, logger, clear_existing=args.clear_existing)
    
    if errors > 0:
        logger.warning(f"Process completed with {errors} errors")
        sys.exit(1)
    else:
        logger.info("Process completed successfully")

if __name__ == "__main__":
    main()
