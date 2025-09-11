#!/bin/bash

# Check if SSL certificates exist and are valid (Let's Encrypt style)
if [ -f "/etc/ssl/certs/fullchain.pem" ] && [ -f "/etc/ssl/private/privkey.pem" ] && [ -s "/etc/ssl/certs/fullchain.pem" ] && [ -s "/etc/ssl/private/privkey.pem" ]; then
    echo "SSL certificates found. Starting nginx with HTTPS support..."
    # Use the full nginx.conf with HTTPS
    nginx -g "daemon off;"
else
    echo "SSL certificates not found or empty. Starting nginx with HTTP only..."
    # Create a temporary nginx config without SSL
    cat > /etc/nginx/nginx.conf << 'EOF'
user nginx;
worker_processes auto;
error_log /var/log/nginx/error.log warn;
pid /var/run/nginx.pid;

events {
    worker_connections 1024;
}

http {
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                    '$status $body_bytes_sent "$http_referer" '
                    '"$http_user_agent" "$http_x_forwarded_for"';

    access_log /var/log/nginx/access.log main;

    sendfile on;
    keepalive_timeout 65;
    client_max_body_size 100M;
    client_body_timeout 300s;
    client_header_timeout 300s;
    proxy_connect_timeout 300s;
    proxy_send_timeout 300s;
    proxy_read_timeout 300s;

    # HTTP Server for direct access
    server {
        listen 8080;
        listen 8443;  # Also listen on 8443 but without SSL for development
        server_name _;

        # Special handling for WASM files FIRST
        location ~ \.wasm$ {
            root /usr/share/nginx/html;
            add_header Content-Type "application/wasm" always;
            add_header Cache-Control "no-cache, no-store, must-revalidate";
            add_header Pragma "no-cache";
            add_header Expires "0";
        }

        # Serve frontend
        location / {
            root /usr/share/nginx/html;
            index index.html;
            try_files $uri $uri/ /index.html;
            
            add_header Cache-Control "no-cache, no-store, must-revalidate";
            add_header Pragma "no-cache";
            add_header Expires "0";
            
            location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg)$ {
                add_header Cache-Control "no-cache, no-store, must-revalidate";
                add_header Pragma "no-cache";
                add_header Expires "0";
            }
        }

        # Proxy API requests to backend
        location /api/ {
            proxy_pass http://127.0.0.1:3000/api/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 300s;
            proxy_connect_timeout 300s;
            proxy_send_timeout 300s;
            client_max_body_size 100M;
            client_body_timeout 300s;
        }
    }
}
EOF
    nginx -g "daemon off;"
fi
