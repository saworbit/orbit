# Orbit Dashboard - Deployment Guide

**Version**: v2.0.0
**Build Status**: ✅ Production Ready
**Last Updated**: 2025-12-06

---

## 📦 Production Build

### Build Output
```
dist/
├── index.html           (0.46 kB, gzipped: 0.29 kB)
├── assets/
│   ├── index-DOxRaMv3.css  (28.57 kB, gzipped: 5.73 kB)
│   └── index--vZPmJIp.js   (340.60 kB, gzipped: 102.38 kB)
└── [static assets]
```

### Build Performance
- **Build Time**: 3.65s
- **Total Bundle Size**: 369.63 kB
- **Gzipped Size**: 108.40 kB
- **Modules Transformed**: 1,806

---

## 🚀 Deployment Options

### Option 1: Orbit Integrated Mode (Recommended)

Deploy the dashboard as part of the Orbit server binary with the `ui` feature flag.

```bash
# Build Orbit server with embedded UI
cd /c/orbit
cargo build --release --features ui

# The server will automatically serve the dashboard from dashboard/dist
# Start the server
./target/release/orbit-server

# Dashboard available at: http://localhost:8080/
```

**Advantages**:
- Single binary deployment
- No separate web server required
- Simplified configuration
- Perfect for desktop applications

**Configuration**:
```toml
# crates/orbit-web/Cargo.toml
[features]
ui = ["tower-http/fs"]  # Include dashboard
```

---

### Option 2: Standalone Static Hosting

Deploy the dashboard separately using any static file server.

#### Using Nginx

```nginx
server {
    listen 80;
    server_name orbit.example.com;
    root /var/www/orbit-dashboard/dist;
    index index.html;

    # SPA routing - always serve index.html
    location / {
        try_files $uri $uri/ /index.html;
    }

    # API proxy to backend
    location /api {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }

    # Gzip compression
    gzip on;
    gzip_types text/css application/javascript application/json;
    gzip_min_length 1000;
}
```

#### Using Apache

```apache
<VirtualHost *:80>
    ServerName orbit.example.com
    DocumentRoot /var/www/orbit-dashboard/dist

    <Directory /var/www/orbit-dashboard/dist>
        Options -Indexes +FollowSymLinks
        AllowOverride All
        Require all granted

        # SPA routing
        RewriteEngine On
        RewriteBase /
        RewriteRule ^index\.html$ - [L]
        RewriteCond %{REQUEST_FILENAME} !-f
        RewriteCond %{REQUEST_FILENAME} !-d
        RewriteRule . /index.html [L]
    </Directory>

    # API proxy
    ProxyPass /api http://localhost:8080/api
    ProxyPassReverse /api http://localhost:8080/api
</VirtualHost>
```

#### Using Node.js (serve)

```bash
# Install serve
npm install -g serve

# Serve the dashboard
cd dashboard/dist
serve -s -p 3000

# Dashboard available at: http://localhost:3000/
```

#### Using Python

```bash
# Python 3
cd dashboard/dist
python -m http.server 3000

# Dashboard available at: http://localhost:3000/
```

---

### Option 3: Docker Deployment

#### Multi-stage Dockerfile for Full Stack

```dockerfile
# Stage 1: Build Rust backend
FROM rust:1.75 as rust-builder
WORKDIR /app
COPY . .
RUN cargo build --release --features ui

# Stage 2: Build React dashboard
FROM node:20-alpine as node-builder
WORKDIR /app
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard/ ./
RUN npm run build

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app

# Copy Rust binary
COPY --from=rust-builder /app/target/release/orbit-server .

# Copy dashboard dist
COPY --from=node-builder /app/dist ./dashboard/dist

# Expose ports
EXPOSE 8080

# Run server with UI
CMD ["./orbit-server"]
```

#### Dashboard-Only Docker Image

```dockerfile
FROM nginx:alpine
COPY dashboard/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

---

### Option 4: Cloud Platforms

#### Vercel

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy
cd dashboard
vercel --prod

# Configure vercel.json for SPA routing
{
  "rewrites": [
    { "source": "/api/:path*", "destination": "http://your-backend.com/api/:path*" },
    { "source": "/(.*)", "destination": "/index.html" }
  ]
}
```

#### Netlify

```bash
# Install Netlify CLI
npm install -g netlify-cli

# Deploy
cd dashboard
netlify deploy --prod

# Configure netlify.toml
[build]
  publish = "dist"
  command = "npm run build"

[[redirects]]
  from = "/api/*"
  to = "http://your-backend.com/api/:splat"
  status = 200

[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

#### AWS S3 + CloudFront

```bash
# Build and sync to S3
cd dashboard
npm run build
aws s3 sync dist/ s3://orbit-dashboard-bucket --delete

# Configure CloudFront for SPA routing
# Set Error Pages: 403 -> /index.html (200)
#                  404 -> /index.html (200)
```

---

## ⚙️ Environment Configuration

### API Endpoint Configuration

The dashboard expects the Orbit API at `http://localhost:8080/api`. To change this:

**Option 1: Environment Variable (Build-time)**

```bash
# .env.production
VITE_API_URL=https://api.orbit.example.com
```

**Option 2: Runtime Configuration**

Modify `dashboard/src/lib/api.ts`:

```typescript
export const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL || "http://localhost:8080/api",
  withCredentials: true,
  headers: {
    "Content-Type": "application/json",
  },
});
```

---

## 🔒 Security Considerations

### Production Checklist

- [ ] **HTTPS Enabled**: Always use HTTPS in production
- [ ] **CORS Configuration**: Configure backend CORS for dashboard domain
- [ ] **Authentication**: Ensure `/api/auth/login` endpoint is secure
- [ ] **Token Expiration**: Implement JWT token refresh mechanism
- [ ] **Content Security Policy**: Add CSP headers
- [ ] **Rate Limiting**: Implement API rate limiting
- [ ] **Secure Cookies**: Use `httpOnly`, `secure`, `sameSite` cookies
- [ ] **Input Validation**: Backend validates all user inputs

### Recommended Headers

```nginx
# Security Headers
add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-Content-Type-Options "nosniff" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "no-referrer-when-downgrade" always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:;" always;
```

---

## 🧪 Pre-Deployment Testing

### 1. Build Verification
```bash
cd dashboard
npm run build
# ✅ Verify: Zero TypeScript errors
# ✅ Verify: Bundle size < 400 kB
```

### 2. Production Preview
```bash
npm run preview
# Test at: http://localhost:4173/
```

### 3. Lighthouse Audit
```bash
# Install Lighthouse CLI
npm install -g lighthouse

# Run audit
lighthouse http://localhost:4173/ --view
# ✅ Target: Performance > 90
# ✅ Target: Accessibility > 95
# ✅ Target: Best Practices > 90
```

### 4. Cross-Browser Testing
- [ ] Chrome/Edge (Chromium)
- [ ] Firefox
- [ ] Safari
- [ ] Mobile browsers (iOS Safari, Chrome Mobile)

---

## 📊 Monitoring & Analytics

### Performance Monitoring

Add performance monitoring to `dashboard/src/main.tsx`:

```typescript
// Web Vitals
import { onCLS, onFID, onLCP } from 'web-vitals';

onCLS(console.log);
onFID(console.log);
onLCP(console.log);
```

### Error Tracking

Integrate Sentry or similar:

```typescript
import * as Sentry from "@sentry/react";

Sentry.init({
  dsn: "YOUR_SENTRY_DSN",
  environment: "production",
  tracesSampleRate: 0.1,
});
```

---

## 🔄 Continuous Deployment

### GitHub Actions Workflow

```yaml
name: Deploy Dashboard

on:
  push:
    branches: [main]
    paths:
      - 'dashboard/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: cd dashboard && npm ci

      - name: Build
        run: cd dashboard && npm run build

      - name: Deploy to S3
        run: aws s3 sync dashboard/dist/ s3://orbit-dashboard --delete
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}

      - name: Invalidate CloudFront
        run: aws cloudfront create-invalidation --distribution-id ${{ secrets.CF_DIST_ID }} --paths "/*"
```

---

## 📝 Rollback Procedure

### Option 1: Git Rollback

```bash
# Identify last working commit
git log --oneline dashboard/

# Checkout previous version
git checkout <commit-hash> dashboard/

# Rebuild
cd dashboard && npm run build

# Redeploy
```

### Option 2: Docker Rollback

```bash
# Tag images with versions
docker tag orbit-dashboard:latest orbit-dashboard:v2.0.0

# Rollback to previous version
docker run -d -p 80:80 orbit-dashboard:v1.9.0
```

---

## 🎯 Post-Deployment Verification

### Health Check Endpoints

```bash
# Dashboard loads
curl -I https://orbit.example.com/

# API connectivity
curl https://orbit.example.com/api/health

# Authentication flow
curl -X POST https://orbit.example.com/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}'
```

### Visual Smoke Tests

- [ ] Login page loads
- [ ] Login with valid credentials redirects to dashboard
- [ ] Dashboard shows KPI cards
- [ ] Transfers screen renders
- [ ] Settings theme toggle works
- [ ] Logout returns to login page

---

## 📚 Additional Resources

- **Development Guide**: dashboard/README.md
- **Test Report**: dashboard/TEST_REPORT.md
- **Changelog**: CHANGELOG.md
- **API Documentation**: docs/API.md (if available)

---

## 🆘 Troubleshooting

### Issue: Dashboard shows blank page

**Solution**:
```bash
# Check browser console for errors
# Verify API URL in dashboard/src/lib/api.ts
# Ensure backend is running and accessible
```

### Issue: Authentication fails

**Solution**:
```bash
# Verify backend /api/auth/login endpoint exists
# Check CORS headers allow dashboard domain
# Verify credentials in backend database
```

### Issue: Chunks not loading

**Solution**:
```bash
# Check network tab for failed asset requests
# Verify base path configuration
# Clear browser cache and rebuild
```

---

**Status**: ✅ Production build ready for deployment
**Build Time**: 3.65s
**Bundle Size**: 340.60 kB (102.38 kB gzipped)
**Next Steps**: Choose deployment option and follow respective guide above
