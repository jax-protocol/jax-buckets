# Templating and UI System - Jax Bucket

## Template Engine: Askama

- Jinja2-like template syntax
- Compile-time template checking
- Type-safe template rendering
- Integration with Axum via `askama_axum`

## Template Structure

### Base Template: `templates/base.html`

All pages extend the base template which provides:
- **Blocks:** `title`, `head`, `content`
- Common head elements (fonts, CSS frameworks, CodeMirror)
- Navigation bar
- Footer
- Dark mode support

### Current Template Hierarchy

```
templates/
├── base.html (parent layout)
├── index.html (node dashboard)
├── buckets.html (bucket list)
├── bucket_explorer.html (file browser)
├── file_viewer.html (file content viewer)
├── file_editor.html (standalone editor - DEPRECATED)
├── bucket_logs.html (history viewer)
├── pins_explorer.html (pin management)
├── peers_explorer.html (peer management)
└── components/
    ├── inline_editor.html (reusable inline editing)
    └── historical_banner.html (read-only version banner)
```

## UI Framework

### Franken UI (Primary)
- Modern UI component library based on UIKit
- CDN-hosted components
- Modal dialogs, tables, forms, grid system

### Custom CSS: `static/style.css`

**Design System:**
- HSL-based CSS variables for theming
- Light/Dark mode support
- Monochromatic palette (black/white/grays)
- Tailwind-like utility classes

**CSS Variables:**
```css
:root {
  --background: 0 0% 100%;
  --foreground: 0 0% 0%;
  --primary: 0 0% 0%;
  --muted: 0 0% 95%;
  /* etc. */
}

.dark {
  --background: 0 0% 0%;
  --foreground: 0 0% 100%;
  /* inverted */
}
```

## Static Assets

### Organization
```
static/
├── 404.html
├── app.js (main JavaScript modules)
├── style.css
└── js/
    └── inline-editor.js (ES module)
```

### Asset Serving: `rust-embed`
- Embeds static files into binary at compile time
- Route: `/static/*path`
- No runtime file system access needed

### External CDN Dependencies
- Inter Font
- Franken UI
- Font Awesome 5
- CodeMirror 6 (ES modules from esm.sh)
- Marked.js (Markdown rendering)

## JavaScript Integration

### Global CodeMirror Loading (base.html)
```html
<script type="module">
  import { EditorView, basicSetup } from "https://esm.sh/codemirror@6.0.1";
  import { markdown } from "https://esm.sh/@codemirror/lang-markdown@6.2.4";

  window.CodeMirror = { EditorView, basicSetup, markdown, oneDark };
</script>
```

### Main JavaScript: `app.js`

**Modules:**
- BucketCreation
- FileUpload
- BucketShare
- FileRename
- FileDelete
- NewFile

**Initialization Pattern:**
```javascript
document.addEventListener("DOMContentLoaded", function() {
  const apiUrl = window.JAX_API_URL || "http://localhost:3000";
  const bucketId = window.JAX_BUCKET_ID;

  BucketCreation.init(apiUrl);
  if (bucketId) {
    FileUpload.init(apiUrl, bucketId);
    // etc.
  }
});
```

### ES6 Modules: `static/js/`

Example: `inline-editor.js`
```javascript
export function initInlineEditor(bucketId, filePath, isMarkdown) { }
export function renderMarkdown(content) { }
```

### Global Configuration Pattern

Templates inject config via window variables:
```html
<script>
window.JAX_API_URL = '{{ api_url }}';
window.JAX_BUCKET_ID = '{{ bucket_id }}';
window.JAX_FILE_PATH = '{{ file_path }}';
window.JAX_IS_MARKDOWN = {{ is_markdown }};
</script>
```

## Routing and Template Rendering

### Route Handler Pattern

**File:** `src/daemon/http_server/html/[handler].rs`

```rust
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{State, Path, Query};

#[derive(Template)]
#[template(path = "template_name.html")]
pub struct TemplateStruct {
    pub field1: String,
    pub field2: Vec<Item>,
}

#[instrument(skip(state))]
pub async fn handler(
    State(state): State<ServiceState>,
) -> askama_axum::Response {
    let template = TemplateStruct { /* ... */ };
    template.into_response()
}
```

### Routes Map

| Route | Handler | Template |
|-------|---------|----------|
| `/` | `index::handler` | `index.html` |
| `/buckets` | `buckets::handler` | `buckets.html` |
| `/buckets/:id` | `bucket_explorer::handler` | `bucket_explorer.html` |
| `/buckets/:id/view` | `file_viewer::handler` | `file_viewer.html` |
| `/buckets/:id/logs` | `bucket_logs::handler` | `bucket_logs.html` |
| `/buckets/:id/pins` | `pins_explorer::handler` | `pins_explorer.html` |
| `/buckets/:id/peers` | `peers_explorer::handler` | `peers_explorer.html` |

## Creating New Pages

### 1. Create Template
```bash
touch crates/app/templates/pages/my_page.html
```

```html
{% extends "layouts/base.html" %}

{% block title %}My Page - Jax{% endblock %}

{% block content %}
<div class="max-w-6xl mx-auto space-y-6">
    <h1 class="text-3xl font-bold">My Page</h1>
</div>
{% endblock %}
```

### 2. Create Handler
```bash
touch crates/app/src/daemon/http_server/html/my_page.rs
```

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "pages/my_page.html")]
pub struct MyPageTemplate {
    pub data: String,
}

pub async fn handler(State(state): State<ServiceState>) -> askama_axum::Response {
    let template = MyPageTemplate { data: "Hello".to_string() };
    template.into_response()
}
```

### 3. Register Route
In `src/daemon/http_server/html/mod.rs`:

```rust
mod my_page;

pub fn router(state: ServiceState) -> Router<ServiceState> {
    Router::new()
        .route("/my-page", get(my_page::handler))
        // ...
}
```

## Creating Reusable Components

### 1. Create Component
```bash
touch crates/app/templates/components/my_component.html
```

```html
<!-- Required parameters: param1, param2 -->
<div class="card">
    <h3>{{ param1 }}</h3>
    <p>{{ param2 }}</p>
</div>
```

### 2. Include in Template
```html
{% block content %}
{% include "components/my_component.html" %}
{% endblock %}
```

### 3. Pass Parameters
Handler provides data in template struct:
```rust
#[derive(Template)]
#[template(path = "pages/parent.html")]
pub struct ParentTemplate {
    pub param1: String,
    pub param2: String,
}
```

## Best Practices

### Template Organization
✅ Single base template for consistency
✅ Components in dedicated directory
✅ Consistent block structure
✅ Breadcrumb navigation

### Styling
✅ CSS variable theming system
✅ Dark mode support
✅ Utility-first CSS
✅ Component-scoped styles

### JavaScript
✅ Module pattern organization
✅ Global configuration via window variables
✅ ES6 modules for new code
✅ Progressive enhancement

### Security
✅ Type-safe templates (compile-time)
✅ Embedded assets (no file system access)
✅ CORS configuration
✅ Read-only mode support

## API Endpoints for Frontend

All API calls go to `/api/v0/bucket/*`:

- `POST /api/v0/bucket/` - Create bucket
- `POST /api/v0/bucket/add` - Add file
- `POST /api/v0/bucket/update` - Update file (expects: bucket_id, mount_path, file)
- `POST /api/v0/bucket/rename` - Rename file
- `POST /api/v0/bucket/delete` - Delete file
- `POST /api/v0/bucket/share` - Share bucket
- `POST /api/v0/bucket/list` - List buckets
- `POST /api/v0/bucket/ls` - List files
- `POST /api/v0/bucket/cat` - Read file

All endpoints expect multipart form data.

## Development Workflow

1. Edit templates in `crates/app/templates/`
2. Edit static assets in `crates/app/static/`
3. Build: `cargo build` (embeds static assets)
4. Run: `cargo run` or restart daemon
5. Templates are compiled at build time, errors caught early

## Historical Version Support

Templates support viewing historical bucket states via `?at={hash}` query parameter:

- `bucket_explorer.html` - Browse files at specific version
- `file_viewer.html` - View file content at specific version
- Historical views are read-only
- Yellow banner component indicates historical mode
- "Return to current version" links provided
