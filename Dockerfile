FROM rust:1.82 as builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build frontend
WORKDIR /app
COPY . .
RUN wasm-pack build --target web

# Build backend and log_decoder
WORKDIR /app/backend
RUN cargo build --release

# Build log_decoder
WORKDIR /app/log_decoder
RUN cargo build --release

# Runtime stage
FROM nginx:1.25

# Install supervisord, python3, pip, cron, and other dependencies
RUN apt-get update && apt-get install -y \
    supervisor \
    python3 \
    python3-pip \
    python3-venv \
    cron \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /app/backend/target/release/fw_log_backend /usr/local/bin/
COPY --from=builder /app/log_decoder/target/release/decoder /usr/local/bin/log_decoder

# Copy frontend files to nginx
COPY --from=builder /app/index.html /usr/share/nginx/html/
COPY --from=builder /app/pkg /usr/share/nginx/html/pkg/

# Copy decoders folder
COPY decoders /app/decoders

# Make decoder files executable
RUN chmod +x /app/decoders/decoder_*

# Copy Azure downloader files
COPY azure_blob_downloader.py /app/
COPY requirements_azure.txt /app/
COPY azure_config_docker.json /app/azure_config.json

# Create virtual environment and install Python dependencies
WORKDIR /app
RUN python3 -m venv venv_azure && \
    . venv_azure/bin/activate && \
    pip install --upgrade pip && \
    pip install -r requirements_azure.txt

# Create directories for logs and downloads
RUN mkdir -p /app/logs /app/downloads

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy startup script
COPY start-nginx.sh /usr/local/bin/start-nginx.sh
RUN chmod +x /usr/local/bin/start-nginx.sh

# Create Azure downloader wrapper script
RUN echo '#!/bin/bash' > /app/run_azure_download.sh && \
    echo 'cd /app' >> /app/run_azure_download.sh && \
    echo 'source venv_azure/bin/activate' >> /app/run_azure_download.sh && \
    echo 'python3 azure_blob_downloader.py >> /app/logs/cron.log 2>&1' >> /app/run_azure_download.sh && \
    chmod +x /app/run_azure_download.sh

# Setup cron job (runs twice daily at 6:00 AM and 6:00 PM)
RUN echo '0 6,18 * * * /app/run_azure_download.sh' > /etc/cron.d/azure-downloader && \
    chmod 0644 /etc/cron.d/azure-downloader && \
    crontab /etc/cron.d/azure-downloader

# Copy supervisor configuration
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Expose port 8080
EXPOSE 8080

# Start supervisor to manage both nginx and backend
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
