# Template Reorganization - Jax Bucket

## New Template Structure

```
crates/app/templates/
├── layouts/
│   └── base.html              # Base layout with nav, footer, head includes
├── pages/
│   ├── index.html            # Node dashboard
│   ├── buckets.html          # Bucket list
│   ├── bucket_explorer.html  # File browser
│   ├── file_viewer.html      # File content viewer
│   ├── file_editor.html      # Standalone editor (DEPRECATED)
│   ├── bucket_logs.html      # History viewer
│   ├── pins_explorer.html    # Pin management
│   └── peers_explorer.html   # Peer management
└── components/
    ├── inline_editor.html     # Reusable inline editing component
    └── historical_banner.html # Read-only version banner
```

## Changes Made

### 1. Directory Structure
- Created `layouts/` for base templates
- Created `pages/` for page templates
- Kept `components/` for reusable components

### 2. Template Paths Updated

All page templates now extend from `layouts/base.html`:
```html
{% extends "layouts/base.html" %}
```

### 3. Rust Handler Updates

All template struct attributes updated to new paths:
```rust
#[template(path = "pages/bucket_explorer.html")]  // was "bucket_explorer.html"
#[template(path = "pages/file_viewer.html")]       // was "file_viewer.html"
// etc.
```

### 4. Files Modified

**Rust handlers:**
- `src/daemon/http_server/html/index.rs`
- `src/daemon/http_server/html/buckets.rs`
- `src/daemon/http_server/html/bucket_explorer.rs`
- `src/daemon/http_server/html/file_viewer.rs`
- `src/daemon/http_server/html/file_editor.rs`
- `src/daemon/http_server/html/bucket_logs.rs`
- `src/daemon/http_server/html/pins_explorer.rs`
- `src/daemon/http_server/html/peers_explorer.rs`

**Templates:**
- All page templates updated to extend `layouts/base.html`

## Benefits

1. **Better Organization**: Clear separation of layouts, pages, and components
2. **Scalability**: Easy to add new layouts or component types
3. **Maintainability**: Logical grouping makes finding templates easier
4. **Conventions**: Follows common templating patterns (Rails, Django, etc.)

## Creating New Templates

### New Page
```bash
touch crates/app/templates/pages/my_page.html
```

```html
{% extends "layouts/base.html" %}

{% block title %}My Page - Jax{% endblock %}

{% block content %}
<!-- Page content -->
{% endblock %}
```

### New Layout
```bash
touch crates/app/templates/layouts/minimal.html
```

```html
<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}{% endblock %}</title>
    {% block head %}{% endblock %}
</head>
<body>
    {% block content %}{% endblock %}
</body>
</html>
```

### New Component
```bash
touch crates/app/templates/components/my_component.html
```

```html
<!-- Component with parameters -->
<div class="component">
    <h3>{{ param1 }}</h3>
    <p>{{ param2 }}</p>
</div>
```

Include it:
```html
{% include "components/my_component.html" %}
```

## Migration Complete

All builds passing, no breaking changes to functionality. The reorganization is complete and ready for production.
