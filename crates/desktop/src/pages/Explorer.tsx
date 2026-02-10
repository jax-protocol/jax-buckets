import { Component, createSignal, onMount, For, Show, createMemo } from 'solid-js';
import { useParams, useSearchParams, useNavigate } from '@solidjs/router';
import { ls, lsAtVersion, mkdir, deletePath, renamePath, uploadNativeFiles, FileEntry } from '../lib/api';
import { pathToBreadcrumbs } from '../lib/utils';
import Breadcrumb from '../components/Breadcrumb';
import ConfirmDialog from '../components/ConfirmDialog';
import SharePanel from '../components/SharePanel';

const Explorer: Component = () => {
  const params = useParams<{ bucketId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();

  const currentPath = () => (searchParams.path as string) || '/';
  const versionHash = () => (searchParams.at as string) || null;
  const isHistoryView = createMemo(() => !!versionHash());

  const [entries, setEntries] = createSignal<FileEntry[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // New folder state
  const [showNewFolder, setShowNewFolder] = createSignal(false);
  const [newFolderName, setNewFolderName] = createSignal('');

  // Rename state
  const [renamingPath, setRenamingPath] = createSignal<string | null>(null);
  const [renameValue, setRenameValue] = createSignal('');

  // Delete confirmation state
  const [deleteTarget, setDeleteTarget] = createSignal<FileEntry | null>(null);

  // Share panel
  const [showSharePanel, setShowSharePanel] = createSignal(false);

  const fetchEntries = async () => {
    try {
      setLoading(true);
      setError(null);
      let result: FileEntry[];
      if (versionHash()) {
        result = await lsAtVersion(params.bucketId, versionHash()!, currentPath());
      } else {
        result = await ls(params.bucketId, currentPath());
      }
      // Sort: folders first, then alphabetically
      result.sort((a, b) => {
        if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
      setEntries(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    fetchEntries();
  });

  // Re-fetch when path changes
  const navigateToPath = (path: string) => {
    if (versionHash()) {
      setSearchParams({ path, at: versionHash()! });
    } else {
      setSearchParams({ path });
    }
    setTimeout(() => fetchEntries(), 0);
  };

  const handleEntryClick = (entry: FileEntry) => {
    if (entry.is_dir) {
      navigateToPath(entry.path);
    } else {
      const atParam = versionHash() ? `&at=${encodeURIComponent(versionHash()!)}` : '';
      navigate(`/buckets/${params.bucketId}/view?path=${encodeURIComponent(entry.path)}${atParam}`);
    }
  };

  const handleUpload = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ multiple: true });
      if (!selected) return;

      const paths = Array.isArray(selected) ? selected as string[] : [selected as string];
      if (paths.length === 0) return;

      setError(null);
      await uploadNativeFiles(params.bucketId, currentPath(), paths);
      await fetchEntries();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleNewFolder = async () => {
    const name = newFolderName().trim();
    if (!name) return;

    try {
      setError(null);
      const path = currentPath().endsWith('/')
        ? `${currentPath()}${name}`
        : `${currentPath()}/${name}`;
      await mkdir(params.bucketId, path);
      setShowNewFolder(false);
      setNewFolderName('');
      await fetchEntries();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRename = async (entry: FileEntry) => {
    const newName = renameValue().trim();
    if (!newName || newName === entry.name) {
      setRenamingPath(null);
      return;
    }

    try {
      setError(null);
      const parent = entry.path.substring(0, entry.path.lastIndexOf('/')) || '/';
      const newPath = parent === '/' ? `/${newName}` : `${parent}/${newName}`;
      await renamePath(params.bucketId, entry.path, newPath);
      setRenamingPath(null);
      await fetchEntries();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async () => {
    const target = deleteTarget();
    if (!target) return;

    try {
      setError(null);
      await deletePath(params.bucketId, target.path);
      setDeleteTarget(null);
      await fetchEntries();
    } catch (e) {
      setError(String(e));
    }
  };

  const goToHead = () => {
    navigate(`/buckets/${params.bucketId}?path=${encodeURIComponent(currentPath())}`);
    setTimeout(() => fetchEntries(), 0);
  };

  const breadcrumbs = () => pathToBreadcrumbs(currentPath());

  return (
    <div>
      {/* History version banner */}
      <Show when={isHistoryView()}>
        <div style={{
          background: 'hsl(217 91% 60% / 0.08)',
          border: '1px solid hsl(217 91% 60% / 0.3)',
          padding: '0.625rem 1rem',
          'border-radius': '8px',
          'margin-bottom': '1rem',
          display: 'flex',
          'justify-content': 'space-between',
          'align-items': 'center',
        }}>
          <span style={{ color: 'var(--accent-blue)', 'font-size': '0.875rem' }}>
            Viewing historical version: <code style={{ 'font-size': '0.75rem' }}>{versionHash()!.substring(0, 16)}...</code>
          </span>
          <button
            onClick={goToHead}
            style={{
              padding: '0.375rem 0.625rem',
              'border-radius': '6px',
              border: '1px solid var(--accent-blue)',
              background: 'transparent',
              color: 'var(--accent-blue)',
              cursor: 'pointer',
              'font-size': '0.75rem',
              'font-family': 'inherit',
              'font-weight': '500',
            }}
          >
            Back to HEAD
          </button>
        </div>
      </Show>

      {/* Header */}
      <div style={{
        display: 'flex',
        'justify-content': 'space-between',
        'align-items': 'center',
        'margin-bottom': '1rem',
      }}>
        <div>
          <h2 style={{ 'font-size': '1.5rem', 'font-weight': '700', 'margin-bottom': '0.5rem' }}>
            Explorer
          </h2>
          <Breadcrumb items={breadcrumbs()} onNavigate={navigateToPath} />
        </div>
        <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)', 'font-family': 'monospace' }}>
          {params.bucketId.substring(0, 8)}...
        </div>
      </div>

      {/* Action bar */}
      <div style={{
        display: 'flex',
        gap: '0.5rem',
        'margin-bottom': '1rem',
      }}>
        <Show when={!isHistoryView()}>
          <button
            onClick={handleUpload}
            style={{
              padding: '0.5rem 0.75rem',
              'border-radius': '8px',
              border: '1px solid var(--border)',
              background: 'var(--fg)',
              color: 'var(--bg)',
              cursor: 'pointer',
              'font-size': '0.8125rem',
              'font-weight': '500',
              'font-family': 'inherit',
            }}
          >
            Upload Files
          </button>
          <button
            onClick={() => { setShowNewFolder(true); setNewFolderName(''); }}
            style={{
              padding: '0.5rem 0.75rem',
              'border-radius': '8px',
              border: '1px solid var(--border)',
              background: 'var(--muted)',
              color: 'var(--fg)',
              cursor: 'pointer',
              'font-size': '0.8125rem',
              'font-weight': '500',
              'font-family': 'inherit',
            }}
          >
            New Folder
          </button>
        </Show>
        <button
          onClick={() => navigate(`/buckets/${params.bucketId}/history`)}
          style={{
            padding: '0.5rem 0.75rem',
            'border-radius': '8px',
            border: '1px solid var(--border)',
            background: 'var(--muted)',
            color: 'var(--fg)',
            cursor: 'pointer',
            'font-size': '0.8125rem',
            'font-weight': '500',
            'font-family': 'inherit',
            'margin-left': isHistoryView() ? '0' : 'auto',
          }}
        >
          History
        </button>
        <Show when={!isHistoryView()}>
          <button
            onClick={() => setShowSharePanel(!showSharePanel())}
            style={{
              padding: '0.5rem 0.75rem',
              'border-radius': '8px',
              border: '1px solid var(--border)',
              background: showSharePanel() ? 'var(--fg)' : 'var(--muted)',
              color: showSharePanel() ? 'var(--bg)' : 'var(--fg)',
              cursor: 'pointer',
              'font-size': '0.8125rem',
              'font-weight': '500',
              'font-family': 'inherit',
            }}
          >
            Share
          </button>
        </Show>
      </div>

      {/* New folder inline input */}
      <Show when={showNewFolder() && !isHistoryView()}>
        <div style={{
          display: 'flex',
          gap: '0.5rem',
          'margin-bottom': '1rem',
          'align-items': 'center',
        }}>
          <input
            type="text"
            placeholder="Folder name..."
            value={newFolderName()}
            onInput={(e) => setNewFolderName(e.currentTarget.value)}
            onKeyPress={(e) => {
              if (e.key === 'Enter') handleNewFolder();
              if (e.key === 'Escape') setShowNewFolder(false);
            }}
            autofocus
            style={{
              padding: '0.375rem 0.625rem',
              'border-radius': '6px',
              border: '1px solid var(--border)',
              background: 'var(--bg)',
              color: 'var(--fg)',
              'font-size': '0.8125rem',
              'font-family': 'inherit',
              outline: 'none',
              width: '200px',
            }}
          />
          <button
            onClick={handleNewFolder}
            disabled={!newFolderName().trim()}
            style={{
              padding: '0.375rem 0.625rem',
              'border-radius': '6px',
              border: '1px solid var(--border)',
              background: 'var(--fg)',
              color: 'var(--bg)',
              cursor: 'pointer',
              'font-size': '0.8125rem',
              'font-family': 'inherit',
              opacity: !newFolderName().trim() ? '0.4' : '1',
            }}
          >
            Create
          </button>
          <button
            onClick={() => setShowNewFolder(false)}
            style={{
              padding: '0.375rem 0.625rem',
              'border-radius': '6px',
              border: '1px solid var(--border)',
              background: 'var(--muted)',
              color: 'var(--fg)',
              cursor: 'pointer',
              'font-size': '0.8125rem',
              'font-family': 'inherit',
            }}
          >
            Cancel
          </button>
        </div>
      </Show>

      {/* Error display */}
      <Show when={error()}>
        <div style={{
          background: 'hsl(0 84% 60% / 0.08)',
          border: '1px solid hsl(0 84% 60% / 0.3)',
          padding: '0.75rem 1rem',
          'border-radius': '8px',
          'margin-bottom': '1rem',
          color: 'var(--accent-red)',
          'font-size': '0.875rem',
        }}>
          {error()}
        </div>
      </Show>

      {/* Loading */}
      <Show when={loading()}>
        <p style={{ color: 'var(--muted-fg)', 'font-size': '0.875rem' }}>Loading...</p>
      </Show>

      {/* Empty state */}
      <Show when={!loading() && entries().length === 0 && !error()}>
        <div style={{
          background: 'var(--muted)',
          border: '1px solid var(--border)',
          'border-radius': 'var(--radius)',
          padding: '3rem',
          'text-align': 'center',
        }}>
          <p style={{ 'font-size': '0.9375rem', 'font-weight': '500', 'margin-bottom': '0.375rem' }}>Empty directory</p>
          <p style={{ color: 'var(--muted-fg)', 'font-size': '0.875rem' }}>
            {isHistoryView() ? 'No files at this version' : 'Upload files or create a folder to get started'}
          </p>
        </div>
      </Show>

      {/* File table */}
      <Show when={!loading() && entries().length > 0}>
        <div style={{
          border: '1px solid var(--border)',
          'border-radius': 'var(--radius)',
          overflow: 'hidden',
        }}>
          {/* Table header */}
          <div style={{
            display: 'grid',
            'grid-template-columns': '1fr 120px 160px',
            padding: '0.625rem 1rem',
            background: 'var(--muted)',
            'border-bottom': '1px solid var(--border)',
            'font-size': '0.75rem',
            'font-weight': '600',
            'text-transform': 'uppercase',
            'letter-spacing': '0.05em',
            color: 'var(--muted-fg)',
          }}>
            <span>Name</span>
            <span>Type</span>
            <span style={{ 'text-align': 'right' }}>Actions</span>
          </div>

          {/* Table rows */}
          <For each={entries()}>
            {(entry) => (
              <div style={{
                display: 'grid',
                'grid-template-columns': '1fr 120px 160px',
                padding: '0.625rem 1rem',
                'border-bottom': '1px solid var(--border)',
                'align-items': 'center',
                transition: 'background 0.1s ease',
              }}>
                {/* Name column */}
                <div style={{ display: 'flex', 'align-items': 'center', gap: '0.5rem', 'min-width': '0' }}>
                  <span style={{ 'font-size': '1rem', 'flex-shrink': '0' }}>
                    {entry.is_dir ? '\u{1F4C1}' : '\u{1F4C4}'}
                  </span>
                  <Show when={renamingPath() === entry.path && !isHistoryView()} fallback={
                    <button
                      onClick={() => handleEntryClick(entry)}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--fg)',
                        cursor: 'pointer',
                        'font-size': '0.875rem',
                        'font-family': 'inherit',
                        'text-align': 'left',
                        padding: '0',
                        overflow: 'hidden',
                        'text-overflow': 'ellipsis',
                        'white-space': 'nowrap',
                      }}
                    >
                      {entry.name}
                    </button>
                  }>
                    <input
                      type="text"
                      value={renameValue()}
                      onInput={(e) => setRenameValue(e.currentTarget.value)}
                      onKeyPress={(e) => {
                        if (e.key === 'Enter') handleRename(entry);
                        if (e.key === 'Escape') setRenamingPath(null);
                      }}
                      onBlur={() => handleRename(entry)}
                      autofocus
                      style={{
                        padding: '0.125rem 0.375rem',
                        'border-radius': '4px',
                        border: '1px solid var(--accent-blue)',
                        background: 'var(--bg)',
                        color: 'var(--fg)',
                        'font-size': '0.875rem',
                        'font-family': 'inherit',
                        outline: 'none',
                        width: '100%',
                      }}
                    />
                  </Show>
                </div>

                {/* Type badge */}
                <div>
                  <span style={{
                    'font-size': '0.6875rem',
                    'font-weight': '500',
                    padding: '0.125rem 0.5rem',
                    'border-radius': '9999px',
                    background: entry.is_dir ? 'hsl(217 91% 60% / 0.12)' : 'var(--muted)',
                    color: entry.is_dir ? 'var(--accent-blue)' : 'var(--muted-fg)',
                  }}>
                    {entry.is_dir ? 'Folder' : entry.mime_type.split('/').pop()}
                  </span>
                </div>

                {/* Actions */}
                <div style={{ display: 'flex', gap: '0.25rem', 'justify-content': 'flex-end' }}>
                  <Show when={!entry.is_dir}>
                    <button
                      onClick={() => handleEntryClick(entry)}
                      style={actionBtnStyle()}
                    >
                      View
                    </button>
                  </Show>
                  <Show when={!isHistoryView()}>
                    <Show when={!entry.is_dir}>
                      <button
                        onClick={() => navigate(`/buckets/${params.bucketId}/edit?path=${encodeURIComponent(entry.path)}`)}
                        style={actionBtnStyle()}
                      >
                        Edit
                      </button>
                    </Show>
                    <button
                      onClick={() => { setRenamingPath(entry.path); setRenameValue(entry.name); }}
                      style={actionBtnStyle()}
                    >
                      Rename
                    </button>
                    <button
                      onClick={() => setDeleteTarget(entry)}
                      style={{
                        ...actionBtnStyle(),
                        color: 'var(--accent-red)',
                      }}
                    >
                      Delete
                    </button>
                  </Show>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* Delete confirmation dialog */}
      <ConfirmDialog
        open={!!deleteTarget()}
        title={`Delete ${deleteTarget()?.is_dir ? 'folder' : 'file'}`}
        message={`Are you sure you want to delete "${deleteTarget()?.name}"? This action cannot be undone.`}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />

      {/* Share panel */}
      <SharePanel
        bucketId={params.bucketId}
        open={showSharePanel()}
        onClose={() => setShowSharePanel(false)}
      />
    </div>
  );
};

function actionBtnStyle(): Record<string, string> {
  return {
    background: 'none',
    border: 'none',
    color: 'var(--muted-fg)',
    cursor: 'pointer',
    'font-size': '0.75rem',
    'font-family': 'inherit',
    padding: '0.25rem 0.375rem',
    'border-radius': '4px',
  };
}

export default Explorer;
