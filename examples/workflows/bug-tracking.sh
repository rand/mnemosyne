#!/usr/bin/env bash
#
# Workflow Example: Bug Tracking and Resolution
#
# This workflow demonstrates how to:
# 1. Document a bug when discovered
# 2. Store the root cause analysis
# 3. Document the fix
# 4. Create a pattern to prevent recurrence
#
# Usage:
#   ./bug-tracking.sh

set -e

PROJECT="myapp"

echo "🐛 Bug Tracking Workflow Example"
echo "================================="
echo ""

# Step 1: Document the bug
echo "Step 1: Documenting the bug..."
BUG_ID=$(mnemosyne remember \
  --content "Bug: User sessions expire immediately on page refresh

             Symptom: Users get logged out every time they refresh the page
             Frequency: 100% reproduction rate
             Severity: Critical - blocks all user workflows
             First reported: 2025-10-27
             Affected versions: v2.1.0 and later" \
  --importance 9 \
  --namespace "project:$PROJECT" \
  --tags "bug,session,critical" \
  --format json | jq -r '.id')

echo "  ✓ Bug documented: $BUG_ID"
echo ""

# Step 2: Document root cause analysis
echo "Step 2: Storing root cause analysis..."
sleep 1  # Brief pause for readability
mnemosyne remember \
  --content "Root Cause: Session expiry bug

             Investigation:
             - Checked Redis TTL: Correctly set to 24 hours
             - Checked session cookie: SameSite=Strict causing issues
             - Browser DevTools: Cookie not sent on same-site requests

             Root Cause:
             Our CDN (different subdomain) serves static assets, making requests
             appear cross-site. SameSite=Strict blocks the session cookie.

             Solution:
             Change SameSite=Strict to SameSite=Lax for session cookies.

             Related to: $BUG_ID" \
  --importance 8 \
  --namespace "project:$PROJECT" \
  --tags "bug,session,root-cause,cookies" \
  --format json > /dev/null

echo "  ✓ Root cause documented"
echo ""

# Step 3: Document the fix
echo "Step 3: Documenting the fix..."
sleep 1
FIX_ID=$(mnemosyne remember \
  --content "Fix: Changed session cookie SameSite attribute

             Changes made:
             - Updated cookie configuration: SameSite=Strict → SameSite=Lax
             - Added comment explaining why Lax is required
             - Updated security documentation

             Code change:
             // session_config.rs
             - .same_site(SameSite::Strict)
             + .same_site(SameSite::Lax)  // Required for CDN subdomain

             Testing:
             - Verified sessions persist across page refreshes
             - Confirmed CSRF protection still works
             - Tested with Safari, Chrome, Firefox

             Resolves: $BUG_ID" \
  --importance 7 \
  --namespace "project:$PROJECT" \
  --tags "fix,session,cookies" \
  --format json | jq -r '.id')

echo "  ✓ Fix documented: $FIX_ID"
echo ""

# Step 4: Create a pattern to prevent recurrence
echo "Step 4: Creating prevention pattern..."
sleep 1
mnemosyne remember \
  --content "Pattern: Cookie SameSite attribute for multi-subdomain apps

             Problem:
             SameSite=Strict blocks cookies from being sent in cross-site contexts,
             including requests from different subdomains.

             Solution:
             Use SameSite=Lax for session cookies when:
             - App uses multiple subdomains (e.g., CDN on cdn.example.com)
             - Need cookies sent on top-level navigation
             - CSRF protection comes from other mechanisms

             Still use SameSite=Strict for:
             - Single-domain applications
             - Security-critical cookies with CSRF risk

             When to apply:
             - Designing authentication systems
             - Configuring session management
             - Setting up CDN or multi-subdomain architecture

             Related bugs: $BUG_ID" \
  --importance 9 \
  --namespace "project:$PROJECT" \
  --tags "pattern,cookies,session,architecture" \
  --format json > /dev/null

echo "  ✓ Prevention pattern created"
echo ""

# Summary
echo "✅ Bug tracking workflow complete!"
echo ""
echo "What we created:"
echo "  1. Bug report with symptoms and severity"
echo "  2. Root cause analysis with investigation details"
echo "  3. Fix documentation with code changes and testing"
echo "  4. Prevention pattern to avoid similar bugs"
echo ""
echo "Benefits:"
echo "  - Future developers can learn from this bug"
echo "  - Pattern prevents recurrence in new features"
echo "  - Complete audit trail from discovery to fix"
echo "  - Searchable by keywords (session, cookies, SameSite)"
echo ""
echo "Try searching:"
echo "  mnemosyne recall --query \"session cookie bug\""
echo "  mnemosyne recall --query \"SameSite pattern\""
echo "  mnemosyne graph --memory-id \"$BUG_ID\" --depth 2"
