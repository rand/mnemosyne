# Mnemosyne Whitepaper Website

This directory contains the website version of the Mnemosyne whitepaper, providing an accessible, visually engaging presentation of the system architecture, capabilities, and validation.

## Overview

- **Single-page application**: All content on one page with smooth scrolling navigation
- **Modern design**: Clean, professional styling with responsive layout
- **Interactive features**: Smooth scroll, progress bar, diagram rendering, copy buttons
- **No build step required**: Static HTML, CSS, and JavaScript (just open and view)
- **Mermaid diagrams**: Automatic rendering via CDN

## Structure

```
website/
├── index.html          # Main HTML file (single-page app)
├── css/
│   └── styles.css      # Modern design system and all styles
├── js/
│   └── main.js         # Interactivity (scroll, nav, animations)
├── assets/
│   └── diagrams/       # Placeholder for exported SVG diagrams (optional)
└── README.md           # This file
```

## Quick Start

### Local Preview

1. **Option 1: Simple file open**
   ```bash
   # Navigate to website directory
   cd docs/whitepaper/website

   # Open in default browser (macOS)
   open index.html

   # Open in default browser (Linux)
   xdg-open index.html

   # Open in default browser (Windows)
   start index.html
   ```

2. **Option 2: Local web server** (recommended for full functionality)
   ```bash
   # Using Python 3
   python3 -m http.server 8000

   # Using Python 2
   python -m SimpleHTTPServer 8000

   # Using Node.js (npx)
   npx serve

   # Using PHP
   php -S localhost:8000
   ```

   Then open: http://localhost:8000

3. **Option 3: VS Code Live Server**
   - Install "Live Server" extension
   - Right-click `index.html` → "Open with Live Server"

## Deployment

### Option 1: GitHub Pages

1. **Enable GitHub Pages**:
   - Go to repository Settings → Pages
   - Source: Deploy from a branch
   - Branch: `main`, Folder: `/docs/whitepaper/website`
   - Save

2. **Access**: `https://USERNAME.github.io/mnemosyne/docs/whitepaper/website/`

3. **Custom domain** (optional):
   - Add `CNAME` file with your domain
   - Configure DNS: `CNAME` record pointing to `USERNAME.github.io`

### Option 2: Netlify

1. **Deploy from Git**:
   ```bash
   # Install Netlify CLI
   npm install -g netlify-cli

   # Navigate to website directory
   cd docs/whitepaper/website

   # Deploy
   netlify deploy --prod
   ```

2. **Drag-and-drop**:
   - Go to https://app.netlify.com/drop
   - Drag the `website/` folder
   - Get instant live URL

3. **Configuration** (`netlify.toml` optional):
   ```toml
   [build]
     publish = "docs/whitepaper/website"
     command = "echo 'No build step required'"

   [[redirects]]
     from = "/*"
     to = "/index.html"
     status = 200
   ```

### Option 3: Vercel

1. **Deploy with Vercel CLI**:
   ```bash
   # Install Vercel CLI
   npm install -g vercel

   # Navigate to website directory
   cd docs/whitepaper/website

   # Deploy
   vercel --prod
   ```

2. **Configuration** (`vercel.json` optional):
   ```json
   {
     "version": 2,
     "public": true,
     "routes": [
       { "src": "/(.*)", "dest": "/$1" }
     ]
   }
   ```

### Option 4: Cloudflare Pages

1. **Connect repository**:
   - Go to https://dash.cloudflare.com/
   - Pages → Create a project → Connect to Git
   - Select repository and branch

2. **Build settings**:
   - Build command: (leave empty)
   - Build output directory: `docs/whitepaper/website`
   - Root directory: `/`

3. **Custom domain**: Add via Cloudflare Pages dashboard

### Option 5: AWS S3 + CloudFront

1. **Create S3 bucket**:
   ```bash
   aws s3 mb s3://mnemosyne-whitepaper
   aws s3 website s3://mnemosyne-whitepaper --index-document index.html
   ```

2. **Upload files**:
   ```bash
   aws s3 sync docs/whitepaper/website s3://mnemosyne-whitepaper --acl public-read
   ```

3. **Configure CloudFront** (optional, for CDN):
   - Create distribution
   - Set origin to S3 bucket
   - Configure SSL/TLS certificate

## Features

### Responsive Design

The website is fully responsive with breakpoints:
- Desktop: > 1024px (full layout, side-by-side)
- Tablet: 640px - 1024px (2-column grid)
- Mobile: < 640px (single column, stacked)

### Interactive Elements

1. **Smooth Scrolling**: Click navigation links for smooth scroll to sections
2. **Progress Bar**: Top progress indicator shows reading progress
3. **Active Nav Highlighting**: Current section highlighted in navigation
4. **Fade-in Animations**: Cards fade in on scroll (respects prefers-reduced-motion)
5. **Mobile Navigation**: Hamburger menu for mobile devices
6. **Copy Buttons**: Click to copy code snippets
7. **Diagram Expansion**: Click diagrams to expand (optional)
8. **Back to Top**: Floating button appears after scrolling
9. **Keyboard Navigation**: ESC closes mobile nav

### Accessibility

- Semantic HTML5 elements
- ARIA labels for interactive elements
- Keyboard navigation support
- High contrast colors (WCAG AA compliant)
- `prefers-reduced-motion` support
- Alt text for images (if added)
- Skip-to-content link (can be added)

### Performance

- **No build step**: Instant deployment
- **CDN for libraries**: Mermaid loaded from jsDelivr CDN
- **Lazy loading**: Images lazy load via Intersection Observer
- **Minimal dependencies**: Only Mermaid.js from CDN
- **Optimized CSS**: Single file, no unused styles
- **Fast load**: Target < 2 seconds

## Customization

### Updating Content

Edit `index.html` directly. Content is organized in semantic sections:
- `<section id="hero">`: Hero/landing section
- `<section id="problem">`: Challenge section
- `<section id="solution">`: Solution/features
- `<section id="architecture">`: Architecture + diagrams
- `<section id="comparison">`: Comparison table
- `<section id="validation">`: Validation + evidence

### Changing Colors

Edit CSS custom properties in `css/styles.css`:
```css
:root {
    --color-primary: #2563eb;        /* Change primary color */
    --color-secondary: #10b981;      /* Change secondary color */
    --color-accent: #f59e0b;         /* Change accent color */
    /* ... */
}
```

### Adding Diagrams

1. **Option 1: Inline Mermaid** (current approach)
   ```html
   <pre class="mermaid">
   graph TD
       A[Start] --> B[End]
   </pre>
   ```

2. **Option 2: SVG export**
   ```bash
   # Export Mermaid diagrams to SVG
   mmdc -i diagrams/01-system-architecture.mmd -o assets/diagrams/01-system-architecture.svg

   # Use in HTML
   <img src="assets/diagrams/01-system-architecture.svg" alt="System Architecture">
   ```

### Adding Analytics

Add Google Analytics, Plausible, or similar:
```html
<!-- Before </head> -->
<script defer data-domain="yourdomain.com" src="https://plausible.io/js/script.js"></script>
```

## Browser Support

- Chrome/Edge: Latest 2 versions ✓
- Firefox: Latest 2 versions ✓
- Safari: Latest 2 versions ✓
- Mobile browsers: iOS Safari 12+, Chrome Mobile ✓

**Features requiring modern browsers**:
- Intersection Observer (scroll animations)
- CSS Grid (layout)
- CSS Custom Properties (theming)
- ES6+ JavaScript (interactivity)

## Troubleshooting

### Mermaid Diagrams Not Rendering

1. **Check browser console** for errors
2. **Network issues**: Ensure CDN is accessible
3. **Syntax errors**: Validate Mermaid syntax at https://mermaid.live
4. **CORS issues**: Use local server instead of file:// protocol

### Styles Not Loading

1. **Check file paths**: Ensure `css/styles.css` exists relative to `index.html`
2. **Clear browser cache**: Hard refresh (Ctrl+Shift+R or Cmd+Shift+R)
3. **Check console**: Look for 404 errors

### Smooth Scroll Not Working

1. **Browser support**: Some older browsers don't support `scroll-behavior: smooth`
2. **JavaScript disabled**: Enable JavaScript in browser
3. **Check navigation links**: Ensure `href` attributes point to valid IDs

## Performance Optimization (Optional)

### Minify Assets

```bash
# Install minifiers
npm install -g csso-cli terser html-minifier

# Minify CSS
csso css/styles.css -o css/styles.min.css

# Minify JS
terser js/main.js -o js/main.min.js -c -m

# Minify HTML
html-minifier --collapse-whitespace --remove-comments index.html -o index.min.html
```

### Generate SVG Diagrams

```bash
# Install mermaid-cli
npm install -g @mermaid-js/mermaid-cli

# Export all diagrams
cd ../diagrams
for file in *.mmd; do
    mmdc -i "$file" -o "../website/assets/diagrams/$(basename "$file" .mmd).svg"
done
```

### Create Favicon

Add to `<head>`:
```html
<link rel="icon" href="favicon.ico" type="image/x-icon">
<link rel="icon" href="favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="apple-touch-icon.png">
```

## Maintenance

### Updating for New Versions

1. Update version number in `index.html`:
   ```html
   <span class="brand-version">v2.1.1</span>
   ```

2. Update footer:
   ```html
   <p class="footer-version">Version 2.1.1 (November 5, 2025)</p>
   ```

3. Update GitHub links if tag changes:
   - Replace all instances of `v2.1.1` with new version
   - Use find-and-replace in editor

### Content Sync

Keep website synchronized with markdown whitepaper:
- Architecture changes → Update diagrams + descriptions
- New features → Add to features grid
- Performance updates → Update stats + validation
- New sections → Add to navigation + content

## License

Same as main repository (MIT). See [LICENSE](https://github.com/USERNAME/mnemosyne/blob/v2.1.1/LICENSE).

## Support

- **Issues**: https://github.com/USERNAME/mnemosyne/issues
- **Discussions**: https://github.com/USERNAME/mnemosyne/discussions
- **Documentation**: https://github.com/USERNAME/mnemosyne/tree/v2.1.1/docs

---

**Last Updated**: 2025-11-06
**Mnemosyne Version**: v2.1.1
**Website Version**: 1.0
