#!/usr/bin/env python3
"""
Generate favicon PNGs and og-image for mnemosyne
Requires: pip install Pillow cairosvg
"""

try:
    from PIL import Image, ImageDraw, ImageFont
    import cairosvg
except ImportError:
    print("Missing dependencies. Install with:")
    print("  pip install Pillow cairosvg")
    exit(1)

# Colors
PINK = "#ff006e"
BG_LIGHT = "#ffffff"
BG_DARK = "#000000"

# Generate PNG favicons from SVG
print("Generating favicon-16x16.png...")
cairosvg.svg2png(
    url="favicon.svg",
    write_to="favicon-16x16.png",
    output_width=16,
    output_height=16
)

print("Generating favicon-32x32.png...")
cairosvg.svg2png(
    url="favicon.svg",
    write_to="favicon-32x32.png",
    output_width=32,
    output_height=32
)

# Generate og-image (1200x630)
print("Generating og-image.png...")
img = Image.new('RGB', (1200, 630), BG_LIGHT)
draw = ImageDraw.Draw(img)

# Try to load a nice font, fall back to default
try:
    title_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Bold.ttf", 72)
    subtitle_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 36)
    symbol_font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 200)
except:
    print("Using default font (install truetype fonts for better results)")
    title_font = ImageFont.load_default()
    subtitle_font = ImageFont.load_default()
    symbol_font = ImageFont.load_default()

# Draw symbol (⊛)
symbol_bbox = draw.textbbox((0, 0), "⊛", font=symbol_font)
symbol_width = symbol_bbox[2] - symbol_bbox[0]
symbol_height = symbol_bbox[3] - symbol_bbox[1]
symbol_x = 150
symbol_y = (630 - symbol_height) // 2
draw.text((symbol_x, symbol_y), "⊛", fill=PINK, font=symbol_font)

# Draw title
title = "Mnemosyne"
title_bbox = draw.textbbox((0, 0), title, font=title_font)
title_x = 450
title_y = 200
draw.text((title_x, title_y), title, fill="#000000", font=title_font)

# Draw subtitle
subtitle = "Semantic Memory & Multi-Agent Orchestration"
subtitle_bbox = draw.textbbox((0, 0), subtitle, font=subtitle_font)
subtitle_x = 450
subtitle_y = 320
draw.text((subtitle_x, subtitle_y), subtitle, fill="#666666", font=subtitle_font)

# Save
img.save("og-image.png")
print("Done! Generated favicon-16x16.png, favicon-32x32.png, and og-image.png")
