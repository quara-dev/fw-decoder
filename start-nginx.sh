#!/bin/bash

# Check if SSL certificates exist
if [ -f "/etc/ssl/certs/fullchain.pem" ] && [ -f "/etc/ssl/certs/privkey.pem" ]; then
    echo "SSL certificates found. Starting nginx with HTTPS support..."
    # Use the full nginx.conf with HTTPS
    nginx -g "daemon off;"
else
    echo "SSL certificates not found. Starting nginx with HTTP only..."
    # Create a temporary nginx config without HTTPS server block
    sed '/# HTTPS Server/,$d' /etc/nginx/nginx.conf > /tmp/nginx-http-only.conf
    echo "}" >> /tmp/nginx-http-only.conf
    nginx -c /tmp/nginx-http-only.conf -g "daemon off;"
fi