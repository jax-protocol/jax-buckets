import { Component, createSignal, onMount, For, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { listBuckets, createBucket, BucketInfo } from '../lib/api';

const Buckets: Component = () => {
  const [buckets, setBuckets] = createSignal<BucketInfo[]>([]);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [newBucketName, setNewBucketName] = createSignal('');
  const [creating, setCreating] = createSignal(false);

  const fetchBuckets = async () => {
    try {
      setLoading(true);
      const result = await listBuckets();
      setBuckets(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCreateBucket = async () => {
    const name = newBucketName().trim();
    if (!name) return;

    try {
      setCreating(true);
      setError(null);
      await createBucket(name);
      setNewBucketName('');
      await fetchBuckets();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

  onMount(() => {
    fetchBuckets();
  });

  return (
    <div>
      <div style={{
        display: 'flex',
        'justify-content': 'space-between',
        'align-items': 'center',
        'margin-bottom': '1.5rem',
      }}>
        <h2 style={{ 'font-size': '1.5rem', 'font-weight': '700' }}>Buckets</h2>
      </div>

      {/* Create bucket form */}
      <div style={{
        display: 'flex',
        gap: '0.5rem',
        'margin-bottom': '1.5rem',
      }}>
        <input
          type="text"
          placeholder="New bucket name..."
          value={newBucketName()}
          onInput={(e) => setNewBucketName(e.currentTarget.value)}
          onKeyPress={(e) => e.key === 'Enter' && handleCreateBucket()}
          style={{
            flex: 1,
            padding: '0.5rem 0.75rem',
            'border-radius': '8px',
            border: '1px solid var(--border)',
            background: 'var(--bg)',
            color: 'var(--fg)',
            'font-size': '0.875rem',
            'font-family': 'inherit',
            outline: 'none',
          }}
        />
        <button
          onClick={handleCreateBucket}
          disabled={creating() || !newBucketName().trim()}
          style={{
            padding: '0.5rem 1rem',
            'border-radius': '8px',
            border: '1px solid var(--border)',
            background: 'var(--fg)',
            color: 'var(--bg)',
            cursor: creating() || !newBucketName().trim() ? 'not-allowed' : 'pointer',
            'font-size': '0.875rem',
            'font-weight': '500',
            'font-family': 'inherit',
            opacity: creating() || !newBucketName().trim() ? '0.4' : '1',
            transition: 'all 0.15s ease',
          }}
        >
          {creating() ? 'Creating...' : 'Create'}
        </button>
      </div>

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

      {/* Loading state */}
      <Show when={loading()}>
        <p style={{ color: 'var(--muted-fg)', 'font-size': '0.875rem' }}>Loading buckets...</p>
      </Show>

      {/* Empty state */}
      <Show when={!loading() && buckets().length === 0}>
        <div style={{
          background: 'var(--muted)',
          border: '1px solid var(--border)',
          'border-radius': 'var(--radius)',
          padding: '3rem',
          'text-align': 'center',
        }}>
          <p style={{ 'font-size': '0.9375rem', 'font-weight': '500', 'margin-bottom': '0.375rem' }}>No buckets yet</p>
          <p style={{ color: 'var(--muted-fg)', 'font-size': '0.875rem' }}>Create your first bucket to get started</p>
        </div>
      </Show>

      {/* Bucket grid */}
      <Show when={!loading() && buckets().length > 0}>
        <div style={{
          display: 'grid',
          'grid-template-columns': 'repeat(auto-fill, minmax(280px, 1fr))',
          gap: '1rem',
        }}>
          <For each={buckets()}>
            {(bucket) => (
              <A
                href={`/buckets/${bucket.bucket_id}?path=/`}
                style={{ 'text-decoration': 'none', color: 'inherit' }}
              >
                <div style={{
                  background: 'var(--muted)',
                  border: '1px solid var(--border)',
                  'border-radius': 'var(--radius)',
                  padding: '1.5rem',
                  transition: 'all 0.2s ease',
                  cursor: 'pointer',
                }}>
                  <div style={{
                    display: 'flex',
                    'justify-content': 'space-between',
                    'align-items': 'flex-start',
                    'margin-bottom': '1rem',
                  }}>
                    <h3 style={{ 'font-size': '0.9375rem', 'font-weight': '600' }}>{bucket.name}</h3>
                    <span style={{
                      'font-size': '0.6875rem',
                      'font-weight': '600',
                      padding: '0.25rem 0.5rem',
                      'border-radius': '9999px',
                      background: 'hsl(142 76% 36% / 0.12)',
                      color: 'var(--accent-green)',
                    }}>
                      Owner
                    </span>
                  </div>

                  <div style={{ display: 'flex', 'flex-direction': 'column', gap: '0.375rem' }}>
                    <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)', 'font-family': 'monospace' }}>
                      {bucket.bucket_id.substring(0, 8)}...
                    </div>
                    <Show when={bucket.link_hash}>
                      <div style={{
                        'font-size': '0.75rem',
                        color: 'var(--muted-fg)',
                        'font-family': 'monospace',
                      }}>
                        {bucket.link_hash.substring(0, 16)}...
                      </div>
                    </Show>
                  </div>

                  <div style={{
                    'margin-top': '1rem',
                    'padding-top': '0.75rem',
                    'border-top': '1px solid var(--border)',
                    display: 'flex',
                    'justify-content': 'space-between',
                    'align-items': 'center',
                  }}>
                    <span style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>
                      {new Date(bucket.created_at).toLocaleDateString()}
                    </span>
                    <span style={{
                      'font-size': '0.75rem',
                      'font-weight': '500',
                      color: 'var(--fg)',
                    }}>
                      Open
                    </span>
                  </div>
                </div>
              </A>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default Buckets;
