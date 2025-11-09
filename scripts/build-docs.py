#!/usr/bin/env python3
"""
Custom documentation build system for mnemosyne.
Parses markdown files and generates static HTML with custom design.
"""

import os
import shutil
from pathlib import Path
import markdown
from jinja2 import Environment, FileSystemLoader
import re

# Project configuration
PROJECT_NAME = "Mnemosyne"
PROJECT_VERSION = "v2.2.0"
PROJECT_TAGLINE = "// Semantic memory for LLM agents"
PROJECT_GLYPH = "⊛"  # Circled asterisk
PROJECT_ACCENT_COLOR = "#ff006e"  # Pink
GITHUB_URL = "https://github.com/rand/mnemosyne"
SITE_URL = "https://rand.github.io/mnemosyne/"

# Directories
BASE_DIR = Path(__file__).parent.parent
DOCS_DIR = BASE_DIR / "docs"
TEMPLATES_DIR = BASE_DIR / "templates"
SITE_DIR = BASE_DIR / "site"

# Navigation structure
NAV_LINKS = [
    {"title": "Whitepaper", "href": "whitepaper.html"},
    {"title": "Abstract", "href": "#abstract"},
    {"title": "Architecture", "href": "#architecture"},
    {"title": "Comparison", "href": "#comparison"},
    {"title": "Validation", "href": "#validation"},
    {"title": "Source", "href": f"{GITHUB_URL}/tree/v2.2.0", "external": True},
]


def setup_markdown():
    """Configure markdown parser with extensions."""
    return markdown.Markdown(
        extensions=[
            "extra",  # Tables, fenced code, etc.
            "codehilite",  # Syntax highlighting
            "toc",  # Table of contents
            "sane_lists",  # Better list handling
        ],
        extension_configs={
            "codehilite": {
                "css_class": "highlight",
                "linenums": False,
            },
            "toc": {
                "permalink": True,
                "permalink_class": "headerlink",
            },
        },
    )


def copy_static_files():
    """Copy CSS, JS, images, and other static assets to site directory."""
    static_dirs = ["css", "js", "assets", "images"]

    for dir_name in static_dirs:
        src_dir = DOCS_DIR / dir_name
        if src_dir.exists():
            dest_dir = SITE_DIR / dir_name
            if dest_dir.exists():
                shutil.rmtree(dest_dir)
            shutil.copytree(src_dir, dest_dir)
            print(f"  Copied {dir_name}/ → site/{dir_name}/")

    # Copy favicon files
    for favicon_file in DOCS_DIR.glob("favicon*"):
        shutil.copy(favicon_file, SITE_DIR / favicon_file.name)
        print(f"  Copied {favicon_file.name}")


def strip_yaml_frontmatter(content):
    """Remove YAML front matter from markdown content."""
    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            return parts[2].strip()
    return content


def render_page(template_env, md_parser, template_name, md_file, output_file, extra_context=None):
    """Render a single page from markdown to HTML."""
    # Read markdown content
    md_path = DOCS_DIR / md_file
    if not md_path.exists():
        print(f"  ⚠️  Skipping {md_file} (not found)")
        return

    with open(md_path, "r", encoding="utf-8") as f:
        md_content = f.read()

    # Strip YAML front matter if present
    md_content = strip_yaml_frontmatter(md_content)

    # Parse markdown to HTML
    html_content = md_parser.convert(md_content)
    toc = md_parser.toc if hasattr(md_parser, "toc") else ""

    # Reset markdown parser for next file
    md_parser.reset()

    # Prepare template context
    context = {
        "project_name": PROJECT_NAME,
        "project_version": PROJECT_VERSION,
        "project_tagline": PROJECT_TAGLINE,
        "project_glyph": PROJECT_GLYPH,
        "github_url": GITHUB_URL,
        "site_url": SITE_URL,
        "nav_links": NAV_LINKS,
        "content": html_content,
        "toc": toc,
    }

    if extra_context:
        context.update(extra_context)

    # Render template
    template = template_env.get_template(template_name)
    html_output = template.render(**context)

    # Write to site directory
    output_path = SITE_DIR / output_file
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(html_output)

    print(f"  ✓ {md_file} → {output_file}")


def build():
    """Main build function."""
    print(f"\n🔨 Building {PROJECT_NAME} documentation...\n")

    # Clean and create site directory
    if SITE_DIR.exists():
        shutil.rmtree(SITE_DIR)
    SITE_DIR.mkdir(parents=True)

    # Setup Jinja2 environment
    template_env = Environment(loader=FileSystemLoader(str(TEMPLATES_DIR)))

    # Setup markdown parser
    md_parser = setup_markdown()

    # Render pages (handle both index.md and INDEX.md)
    print("Rendering pages:")

    # Try both lowercase and uppercase index
    index_file = "index.md" if (DOCS_DIR / "index.md").exists() else "INDEX.md"
    render_page(template_env, md_parser, "index.html", index_file, "index.html")

    render_page(template_env, md_parser, "whitepaper.html", "whitepaper.md", "whitepaper.html")

    # Copy static files
    print("\nCopying static assets:")
    copy_static_files()

    # Create .nojekyll file to disable GitHub Pages Jekyll processing
    (SITE_DIR / ".nojekyll").touch()
    print("  ✓ Created .nojekyll")

    print(f"\n✅ Build complete! Site generated in: {SITE_DIR}\n")
    print(f"To preview locally:")
    print(f"  cd {SITE_DIR}")
    print(f"  python -m http.server 8000\n")


if __name__ == "__main__":
    build()
