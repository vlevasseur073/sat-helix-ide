# sat-helix-ide Documentation

This directory contains the mdbook-based documentation for sat-helix-ide.

## Structure

```text
book/
├── book.toml             # mdbook configuration
├── README.md             # This file
├── src/                 # Source markdown files
│   ├── SUMMARY.md        # Table of contents
│   ├── introduction.md    # Introduction
│   ├── quick-start.md     # Quick Start guide
│   ├── user-manual.md     # User Manual
│   ├── screenshots.md     # Screenshots (placeholders)
│   ├── configuration.md   # Configuration reference
│   ├── architecture.md    # Architecture & Design
│   ├── development.md     # Development guide
│   ├── troubleshooting.md  # Troubleshooting guide
│   └── appendix.md        # Appendix & Reference
└── theme/                # Custom theme files
    ├── custom.css        # Custom CSS
    └── custom.js         # Custom JavaScript
```

## Building the Book

### Prerequisites

- [mdbook](https://github.com/rust-lang/mdBook) v0.5.4 or later

### Build Command

```bash
mdbook build
```

The built book will be in the `book/` directory.

### Serve Locally

```bash
mdbook serve --open
```

This will serve the book at `http://localhost:3000` and open it in your browser.

### Clean Build

```bash
rm -rf book/book
```

## GitHub Pages Deployment

The book is automatically deployed to GitHub Pages via the GitHub Actions workflow:

- **Workflow**: `.github/workflows/pages.yml`
- **Trigger**: Pushes to `main` or `doc` branches
- **URL**: https://vlevasseur073.github.io/sat-helix-ide/

### Manual Deployment

To manually trigger a deployment:

1. Push changes to `main` or `doc` branch
2. Or go to GitHub Actions and run the "GitHub Pages" workflow manually

## Adding Content

1. Create a new markdown file in `src/`
2. Add it to `SUMMARY.md`
3. Test locally with `mdbook serve`
4. Commit and push to trigger deployment

## Adding Screenshots

The screenshots in `screenshots.md` are currently placeholders. To add actual screenshots:

1. Create an `assets/` directory in `src/`:
   ```bash
   mkdir -p src/assets
   ```
2. Add your screenshots (PNG recommended)
3. Update the image paths in `screenshots.md`
4. Reference them like: `![Description](assets/filename.png)`

### Recommended Screenshots

- `initial-workspace.png` - Default layout after init
- `yazi-fullscreen.png` - Yazi in full-screen mode
- `yazi-docked.png` - Yazi docked to the left
- `terminal-visible.png` - Terminal visible
- `terminal-hidden.png` - Terminal hidden
- `terminal-zoomed.png` - Terminal zoomed
- `lazygit-integration.png` - Lazygit in action
- `development-workflow.png` - Full development setup
- `review-workflow.png` - Review tool in action

### Tips for Good Screenshots

- Use a clean, readable font (Fira Code, JetBrains Mono)
- Ensure good contrast
- Show realistic content
- Keep terminal at reasonable size
- Maintain consistent styling

## Customization

### CSS Styling

Custom CSS is in `theme/custom.css`. The book uses:
- Rust/mdbook default theme
- Custom CSS for styling improvements
- Responsive design for mobile

### JavaScript Enhancements

Custom JavaScript is in `theme/custom.js`. Features include:
- Mermaid diagram support
- Smooth scrolling
- Keyboard navigation (Left/Right arrows, H/L)
- Touch support for mobile
- Code block copy buttons
- Section highlighting in sidebar

### Configuration

The `book.toml` file contains mdbook configuration:

- Book metadata (title, authors)
- HTML output settings
- Git repository URL
- Custom theme files

## Writing Documentation

### Standards

- Use clear, concise language
- Include examples for configuration
- Document all user-facing features
- Link to related documentation
- Use consistent formatting

### Markdown Features

- Use Rust/mdbook's rust syntax highlighting
- Tables for structured data
- Code blocks for examples
- Mermaid diagrams for flowcharts

### Example Mermaid Diagram

```mermaid
flowchart TB
    A[Start] --> B{Decision}
    B -->|Yes| C[Do Something]
    B -->|No| D[Do Something Else]
    C --> E[End]
    D --> E
```

## Resources

- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [mdBook GitHub](https://github.com/rust-lang/mdBook)
- [Mermaid Documentation](https://mermaid.js.org/)
- [GitHub Pages Documentation](https://pages.github.com/)

## License

This documentation is part of sat-helix-ide and is licensed under the MIT License.
