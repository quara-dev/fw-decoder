# HTTPS Setup with Cloudflare

This guide explains how to configure HTTPS for your firmware decoder application when using Cloudflare DNS.

## Overview

The current configuration supports two HTTPS setup approaches:

1. **Cloudflare Proxied** (Recommended) - Free SSL with Cloudflare's edge certificates
2. **Origin Certificates** - Better security with end-to-end encryption

## Option 1: Cloudflare Proxied (Easiest)

### 1. Cloudflare DNS Setup

1. Add your domain to Cloudflare
2. Point your A record to your server's IP:
   ```
   Type: A
   Name: @ (or subdomain like 'fw-decoder')
   Content: YOUR_SERVER_IP
   Proxy status: Proxied (orange cloud)
   TTL: Auto
   ```

### 2. Cloudflare SSL/TLS Settings

1. Go to SSL/TLS tab in Cloudflare dashboard
2. Set encryption mode to **"Flexible"** or **"Full"**
   - **Flexible**: Cloudflare ↔ Visitor (HTTPS), Cloudflare ↔ Server (HTTP)
   - **Full**: Cloudflare ↔ Visitor (HTTPS), Cloudflare ↔ Server (HTTPS with self-signed cert)

### 3. Generate Self-Signed Certificates (for Full mode)

```bash
# Create SSL directory
sudo mkdir -p /path/to/ssl

# Generate self-signed certificate
sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout /path/to/ssl/privkey.pem \
    -out /path/to/ssl/fullchain.pem \
    -subj "/C=US/ST=State/L=City/O=Organization/CN=your-domain.com"

# Set proper permissions
sudo chmod 600 /path/to/ssl/privkey.pem
sudo chmod 644 /path/to/ssl/fullchain.pem
```

### 4. Update docker-compose.yml

Ensure your volume mapping points to the correct SSL directory:

```yaml
volumes:
  - /path/to/ssl:/etc/ssl/certs:ro
```

### 5. Deploy

```bash
docker-compose down
docker-compose up -d --build
```

Your site will be accessible at `https://your-domain.com`

## Option 2: Cloudflare Origin Certificates (Most Secure)

### 1. Generate Origin Certificate

1. Go to Cloudflare Dashboard → SSL/TLS → Origin Server
2. Click "Create Certificate"
3. Choose:
   - Let Cloudflare generate a private key and a CSR
   - Hostnames: `your-domain.com`, `*.your-domain.com`
   - Certificate Validity: 15 years
4. Copy the certificate and private key

### 2. Save Certificates

```bash
# Create SSL directory
sudo mkdir -p /path/to/ssl

# Save the origin certificate
sudo tee /path/to/ssl/fullchain.pem > /dev/null <<EOF
-----BEGIN CERTIFICATE-----
[PASTE YOUR ORIGIN CERTIFICATE HERE]
-----END CERTIFICATE-----
EOF

# Save the private key
sudo tee /path/to/ssl/privkey.pem > /dev/null <<EOF
-----BEGIN PRIVATE KEY-----
[PASTE YOUR PRIVATE KEY HERE]
-----END PRIVATE KEY-----
EOF

# Set proper permissions
sudo chmod 600 /path/to/ssl/privkey.pem
sudo chmod 644 /path/to/ssl/fullchain.pem
```

### 3. Cloudflare SSL/TLS Settings

1. Set encryption mode to **"Full (strict)"**
2. Enable "Always Use HTTPS"
3. Set minimum TLS version to 1.2

### 4. Deploy

```bash
docker-compose down
docker-compose up -d --build
```

## Option 3: Let's Encrypt (Alternative)

If you prefer Let's Encrypt certificates:

### 1. Install Certbot

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install certbot

# Stop your current containers to free up ports
docker-compose down
```

### 2. Generate Certificate

```bash
# Generate certificate (replace with your domain)
sudo certbot certonly --standalone -d your-domain.com

# Certificates will be stored in:
# /etc/letsencrypt/live/your-domain.com/fullchain.pem
# /etc/letsencrypt/live/your-domain.com/privkey.pem
```

### 3. Update docker-compose.yml

```yaml
volumes:
  - /etc/letsencrypt/live/your-domain.com:/etc/ssl/certs:ro
```

### 4. Setup Auto-Renewal

```bash
# Add to crontab
sudo crontab -e

# Add this line for auto-renewal
0 12 * * * /usr/bin/certbot renew --quiet && docker-compose -f /path/to/your/docker-compose.yml restart nginx
```

## Troubleshooting

### Check SSL Certificate

```bash
# Test SSL configuration
openssl s_client -connect your-domain.com:443 -servername your-domain.com

# Check certificate expiry
echo | openssl s_client -connect your-domain.com:443 2>/dev/null | openssl x509 -noout -dates
```

### Common Issues

1. **502 Bad Gateway**: Backend service not running
   ```bash
   docker-compose logs backend
   ```

2. **SSL Certificate Error**: Wrong certificate paths or permissions
   ```bash
   docker-compose logs nginx
   ```

3. **Connection Refused**: Firewall blocking ports
   ```bash
   # Allow HTTPS port
   sudo ufw allow 443
   ```

### Nginx Logs

```bash
# Check nginx logs
docker-compose logs nginx

# Check backend logs
docker-compose logs backend
```

## Security Considerations

1. **Use Full (strict) mode** with Cloudflare for end-to-end encryption
2. **Enable HSTS** in Cloudflare → SSL/TLS → Edge Certificates
3. **Enable "Always Use HTTPS"** to redirect HTTP to HTTPS
4. **Set minimum TLS version** to 1.2 or higher
5. **Regular certificate renewal** for Let's Encrypt certificates

## Final Configuration

After following this guide, your application will be accessible at:

- `https://your-domain.com` (HTTPS, secured)
- `http://your-domain.com` (redirects to HTTPS)

The nginx configuration automatically:
- Redirects all HTTP traffic to HTTPS
- Serves the frontend on HTTPS
- Proxies API requests to the backend securely
- Handles WebAssembly files with correct MIME types
