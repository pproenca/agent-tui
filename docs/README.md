# Documentation Site

This directory contains the agent-tui documentation, built with [Mintlify](https://mintlify.com).

## Local Development

### Install Mintlify CLI

```bash
npm i -g mintlify
```

### Run Locally

```bash
cd docs
mintlify dev
```

This starts a local server at `http://localhost:3000`.

## Deployment

### Mintlify Cloud (Recommended)

1. Go to [Mintlify Dashboard](https://dashboard.mintlify.com)
2. Connect your GitHub repository
3. Select the `docs` directory
4. Deploy automatically on push to main

### Self-Hosted

Mintlify can also be self-hosted. See [Mintlify docs](https://mintlify.com/docs/development) for details.

## Structure

```
docs/
├── mint.json              # Mintlify configuration
├── favicon.svg            # Site favicon
├── logo/
│   ├── light.svg          # Logo for light mode
│   └── dark.svg           # Logo for dark mode
├── images/                # Documentation images
├── introduction.mdx       # Home page
├── quickstart.mdx         # Quick start guide
├── commands/              # Command reference
│   ├── overview.mdx
│   ├── spawn.mdx
│   ├── snapshot.mdx
│   └── ...
├── guides/                # How-to guides
│   ├── claude-code-automation.mdx
│   ├── project-wizards.mdx
│   └── element-detection.mdx
└── ai-integration/        # AI agent best practices
    ├── overview.mdx
    ├── optimal-workflow.mdx
    └── troubleshooting.mdx
```

## Adding Pages

1. Create a new `.mdx` file in the appropriate directory
2. Add frontmatter:
   ```yaml
   ---
   title: Page Title
   description: Short description
   ---
   ```
3. Add the page to `mint.json` navigation

## Updating Content

- Edit the `.mdx` files directly
- Use [MDX components](https://mintlify.com/docs/components) for rich content
- Changes auto-reload in development mode

## Custom Images

1. Add images to `docs/images/`
2. Reference in MDX:
   ```jsx
   <img src="/images/your-image.png" alt="Description" />
   ```

## Resources

- [Mintlify Documentation](https://mintlify.com/docs)
- [MDX Guide](https://mdxjs.com/docs/)
- [Component Reference](https://mintlify.com/docs/components)
