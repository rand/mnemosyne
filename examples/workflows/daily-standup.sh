#!/usr/bin/env bash
#
# Workflow Example: Daily Standup Preparation
#
# This workflow demonstrates how to:
# 1. Recall yesterday's work
# 2. Store today's plan
# 3. Track blockers
# 4. Share context with team
#
# Usage:
#   ./daily-standup.sh

set -e

PROJECT="myapp"
TODAY=$(date +%Y-%m-%d)
YESTERDAY=$(date -v-1d +%Y-%m-%d 2>/dev/null || date -d "yesterday" +%Y-%m-%d)

echo "📅 Daily Standup Preparation"
echo "============================"
echo "Date: $TODAY"
echo ""

# Step 1: Recall yesterday's work
echo "Step 1: Recalling yesterday's work..."
echo ""
echo "🔍 Searching memories from $YESTERDAY..."
mnemosyne recall \
  --query "work progress implementation" \
  --namespace "project:$PROJECT" \
  --min-importance 5 \
  --limit 10 \
  --format json | \
  jq -r '.results[] | "  • [\(.importance)/10] \(.summary)"'

echo ""
echo "  💡 Tip: Look for recent commits too:"
echo "     git log --since=\"$YESTERDAY\" --oneline --author=\"\$(git config user.email)\""
echo ""

# Step 2: Store today's plan
echo "Step 2: Storing today's plan..."
PLAN_ID=$(mnemosyne remember \
  --content "Daily Plan: $TODAY

             Today's Goals:
             1. Complete authentication refactor
                - Extract JWT validation into middleware
                - Add refresh token rotation
                - Update tests

             2. Review Sarah's database migration PR
                - Check schema changes
                - Verify rollback script
                - Test locally

             3. Update API documentation
                - Document new auth endpoints
                - Add examples for token refresh

             Expected blockers:
             - May need DevOps help with Redis config
             - Waiting on design review for new login flow" \
  --importance 7 \
  --namespace "project:$PROJECT" \
  --tags "plan,daily,standup" \
  --format json | jq -r '.id')

echo "  ✓ Today's plan stored: $PLAN_ID"
echo ""

# Step 3: Check for blockers
echo "Step 3: Checking for active blockers..."
echo ""
echo "🚧 Recent blockers and issues:"
mnemosyne recall \
  --query "blocker issue blocked waiting" \
  --namespace "project:$PROJECT" \
  --min-importance 6 \
  --limit 5 \
  --format json | \
  jq -r '.results[] | "  ⚠️  [\(.importance)/10] \(.summary)"'

echo ""

# Step 4: Generate standup summary
echo "Step 4: Generating standup summary..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 STANDUP SUMMARY - $TODAY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ YESTERDAY:"
mnemosyne recall \
  --query "completed implemented finished" \
  --namespace "project:$PROJECT" \
  --min-importance 6 \
  --limit 3 \
  --format json | \
  jq -r '.results[] | "  • \(.summary)"'

echo ""
echo "🎯 TODAY:"
echo "  • Complete authentication refactor"
echo "  • Review database migration PR"
echo "  • Update API documentation"

echo ""
echo "🚧 BLOCKERS:"
mnemosyne recall \
  --query "blocker blocked waiting" \
  --namespace "project:$PROJECT" \
  --min-importance 7 \
  --limit 2 \
  --format json | \
  jq -r '.results[] | "  • \(.summary)"' || echo "  • None"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Step 5: Export for team sharing
echo "Step 5: Exporting for team sharing..."
EXPORT_FILE="standup-$TODAY.md"
mnemosyne export \
  --namespace "project:$PROJECT" \
  --output "$EXPORT_FILE" \
  --min-importance 6

echo "  ✓ Team context exported to: $EXPORT_FILE"
echo ""

# Summary
echo "✅ Standup preparation complete!"
echo ""
echo "Benefits:"
echo "  - Quick recall of yesterday's work"
echo "  - Documented plan for today"
echo "  - Identified blockers early"
echo "  - Generated shareable summary"
echo "  - Exported context for team"
echo ""
echo "Integration tips:"
echo "  - Run this script every morning"
echo "  - Add to your shell startup (~/.bashrc)"
echo "  - Create a git alias: git standup"
echo "  - Combine with 'git log --since=yesterday'"
echo ""
echo "Try customizing:"
echo "  - Change PROJECT variable to your project name"
echo "  - Adjust importance thresholds"
echo "  - Add project-specific queries"
echo "  - Export to Slack/email format"
