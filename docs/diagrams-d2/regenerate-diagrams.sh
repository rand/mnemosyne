#!/bin/bash
# Regenerate all mnemosyne diagrams with light/dark variants

set -e

cd "$(dirname "$0")"
mkdir -p ../assets/diagrams

echo "Regenerating mnemosyne diagrams..."

# Process diagrams that already have explicit light/dark variants
for diagram in *-light.d2; do
  if [ -f "$diagram" ]; then
    basename=$(basename "$diagram" .d2)
    echo "  Rendering: $basename"
    d2 "$diagram" "../assets/diagrams/${basename}.svg"
  fi
done

for diagram in *-dark.d2; do
  if [ -f "$diagram" ]; then
    basename=$(basename "$diagram" .d2)
    echo "  Rendering: $basename"
    d2 "$diagram" "../assets/diagrams/${basename}.svg"
  fi
done

# Process base diagrams (no theme suffix) - generate both light and dark
for diagram in *.d2; do
  basename=$(basename "$diagram" .d2)

  # Skip if this is a theme variant
  if [[ "$basename" == *-light ]] || [[ "$basename" == *-dark ]]; then
    continue
  fi

  # Skip non-diagram files
  if [[ "$basename" == "README" ]]; then
    continue
  fi

  echo "  Rendering: $basename (light + dark)"
  d2 --theme=0 "$diagram" "../assets/diagrams/${basename}-light.svg"
  d2 --theme=200 "$diagram" "../assets/diagrams/${basename}-dark.svg"
done

echo "Done! Generated diagrams:"
ls -lh ../assets/diagrams/
