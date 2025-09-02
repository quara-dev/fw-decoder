# Deployment

## Building and Running with Docker

### Prerequisites
- Docker and Docker Compose installed
- Your decoder executables in the `decoders/` folder

### Build and Run
```bash
# Build and start the container
docker-compose up --build

# Or run in detached mode
docker-compose up --build -d

# View logs
docker-compose logs -f

# Stop the container
docker-compose down
```

The application will be available at `http://localhost` (port 80).

### Production Deployment

For production deployment on a server:

1. **Set up your server** with Docker and Docker Compose
2. **Copy your project** to the server
3. **Ensure decoders are executable**:
   ```bash
   chmod +x decoders/*
   ```
4. **Run the container**:
   ```bash
   docker-compose up --build -d
   ```

### SSL/HTTPS Setup

For production with SSL, modify the `docker-compose.yml` to add a reverse proxy like Traefik or add SSL certificates to nginx.

Example with custom domain:
```yaml
services:
  fw_log_decoder:
    build: .
    ports:
      - "80:80"
      - "443:443"  # For HTTPS
    volumes:
      - ./decoders:/app/decoders:ro
      - ./ssl:/etc/nginx/ssl:ro  # Mount SSL certificates
```

### Environment Variables

You can customize the deployment by setting environment variables in `docker-compose.yml`:

```yaml
environment:
  - RUST_LOG=debug  # For more verbose logging
  - BACKEND_PORT=8080  # Backend port (internal)
```

### File Upload Limits

The nginx configuration allows up to 100MB file uploads. To change this, modify `nginx.conf`:

```nginx
client_max_body_size 500M;  # Allow 500MB uploads
```

### Troubleshooting

1. **Check container logs**:
   ```bash
   docker-compose logs fw_log_decoder
   ```

2. **Access container shell**:
   ```bash
   docker-compose exec fw_log_decoder sh
   ```

3. **Verify decoders are accessible**:
   ```bash
   docker-compose exec fw_log_decoder ls -la /app/decoders
   ```

4. **Test backend directly**:
   ```bash
   curl http://localhost/api/versions
   ```
