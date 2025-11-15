# Final Template Structure - Jax Bucket

## Directory Organization

```
crates/app/templates/
├── layouts/
│   └── base.html              # Base layout with nav, footer, CodeMirror, etc.
├── pages/
│   ├── index.html            # Bucket list (/ and /buckets)
│   └── buckets/              # Bucket-specific pages (/buckets/:id/*)
│       ├── index.html        # Bucket file browser (/buckets/:id)
│       ├── viewer.html       # File viewer (/buckets/:id/view)
│       ├── logs.html         # History viewer (/buckets/:id/logs)
│       ├── pins.html         # Pin management (/buckets/:id/pins)
│       └── peers.html        # Peer management (/buckets/:id/peers)
└── components/
    ├── inline_editor.html     # Reusable inline editing component
    └── historical_banner.html # Read-only version banner
```

## URL Structure Matches Template Structure

| Route | Template | Description |
|-------|----------|-------------|
| `/` | `pages/index.html` | List all buckets (home page) |
| `/buckets` | `pages/index.html` | List all buckets |
| `/buckets/:id` | `pages/buckets/index.html` | Browse bucket files (bucket home) |
| `/buckets/:id/view` | `pages/buckets/viewer.html` | View file content |
| `/buckets/:id/logs` | `pages/buckets/logs.html` | Browse bucket history |
| `/buckets/:id/pins` | `pages/buckets/pins.html` | Manage pins |
| `/buckets/:id/peers` | `pages/buckets/peers.html` | Manage peers |

## Design Rationale

### Logical Grouping
- **Root page** (`pages/index.html`) shows bucket list - this is the entry point
- **Bucket-specific pages** nested in `/pages/buckets/` mirror the `/buckets/:id/*` URL structure
- `buckets/index.html` is the bucket explorer because `/buckets/:id` is the index page for a bucket

### Index Files Pattern
- `pages/index.html` → Entry point for the app (bucket list)
- `pages/buckets/index.html` → Entry point for a specific bucket (file browser)
- This mirrors standard web conventions where `index.html` is the default page

### No Redundant Naming
- ❌ `bucket_explorer.html` → ✅ `buckets/index.html`
- ❌ `bucket_logs.html` → ✅ `logs.html` (in buckets/ folder)
- ❌ `file_viewer.html` → ✅ `viewer.html` (in buckets/ folder)
- ❌ `peers_explorer.html` → ✅ `peers.html` (in buckets/ folder)

Context is provided by folder structure, not file naming.

### Deprecated/Removed
- `file_editor.html` - Removed (replaced by inline editor in viewer.html)
- `/buckets/:id/edit` route - Removed
- Node dashboard - Removed (entry point is now bucket list)

## Handler Mappings

**Root page:**
```rust
// src/daemon/http_server/html/buckets.rs
#[template(path = "pages/index.html")]
// Used for both "/" and "/buckets" routes
```

**Bucket-specific pages:**
```rust
// src/daemon/http_server/html/bucket_explorer.rs
#[template(path = "pages/buckets/index.html")]

// src/daemon/http_server/html/file_viewer.rs
#[template(path = "pages/buckets/viewer.html")]

// src/daemon/http_server/html/bucket_logs.rs
#[template(path = "pages/buckets/logs.html")]

// src/daemon/http_server/html/pins_explorer.rs
#[template(path = "pages/buckets/pins.html")]

// src/daemon/http_server/html/peers_explorer.rs
#[template(path = "pages/buckets/peers.html")]
```

## Extending the Structure

### Adding a new root page
```bash
touch crates/app/templates/pages/settings.html
```

```rust
// src/daemon/http_server/html/settings.rs
#[template(path = "pages/settings.html")]
```

### Adding a new bucket-specific page
```bash
touch crates/app/templates/pages/buckets/stats.html
```

```rust
// src/daemon/http_server/html/bucket_stats.rs
#[template(path = "pages/buckets/stats.html")]
```

Route: `/buckets/:bucket_id/stats`

### Adding a new layout
```bash
touch crates/app/templates/layouts/minimal.html
```

Use in templates:
```html
{% extends "layouts/minimal.html" %}
```

### Adding a new component
```bash
touch crates/app/templates/components/file_card.html
```

Include in pages:
```html
{% include "components/file_card.html" %}
```

## Benefits of This Structure

1. **URL-to-File Mapping**: Clear relationship between routes and templates
2. **Logical Nesting**: Bucket pages grouped together, clear hierarchy
3. **No Redundancy**: Folder context eliminates need for prefixes
4. **Scalable**: Easy to add new pages at appropriate levels
5. **Maintainable**: Find files by thinking about URL structure

## Migration Summary

- ✅ Organized templates into layouts/pages/components
- ✅ Nested bucket-specific pages under pages/buckets/
- ✅ Removed redundant naming (bucket_, _explorer suffixes)
- ✅ Deleted deprecated file_editor
- ✅ Updated all Rust handler template paths
- ✅ Updated all template extends directives
- ✅ All builds passing
