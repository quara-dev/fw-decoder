FROM rust:1.81 as builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build frontend
WORKDIR /app
COPY . .
RUN wasm-pack build --target web

# Build backend
WORKDIR /app/backend
RUN cargo build --release

# Runtime stage
FROM nginx:1.25

# Install supervisord to run multiple processes
RUN apt-get update && apt-get install -y supervisor && rm -rf /var/lib/apt/lists/*

# Copy built backend binary
COPY --from=builder /app/backend/target/release/fw_log_backend /usr/local/bin/

# Copy frontend files to nginx
COPY --from=builder /app/index.html /usr/share/nginx/html/
COPY --from=builder /app/pkg /usr/share/nginx/html/pkg/

# Copy decoders folder
COPY decoders /app/decoders

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy startup script
COPY start-nginx.sh /usr/local/bin/start-nginx.sh
RUN chmod +x /usr/local/bin/start-nginx.sh

# Copy supervisor configuration
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Expose port 8080
EXPOSE 8080

# Start supervisor to manage both nginx and backend
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
