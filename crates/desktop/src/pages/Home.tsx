import { Component, createSignal, onMount, onCleanup, Show } from 'solid-js';
import { getStatus, DaemonStatus } from '../lib/api';

const Home: Component = () => {
  const [status, setStatus] = createSignal<DaemonStatus | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);

  let interval: ReturnType<typeof setInterval>;

  const fetchStatus = async () => {
    try {
      const result = await getStatus();
      setStatus(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    fetchStatus();
    interval = setInterval(fetchStatus, 5000);
  });

  onCleanup(() => clearInterval(interval));

  return (
    <div style={{ 'max-width': '640px' }}>
      <h2 style={{ 'font-size': '1.5rem', 'font-weight': '700', 'margin-bottom': '1.5rem' }}>
        Home
      </h2>

      {/* Node Status */}
      <div style={{
        background: 'var(--muted)',
        border: '1px solid var(--border)',
        'border-radius': 'var(--radius)',
        padding: '1.5rem',
        'margin-bottom': '1rem',
      }}>
        <h3 style={sectionHeaderStyle()}>Node Status</h3>

        <Show when={loading()}>
          <p style={{ color: 'var(--muted-fg)', 'font-size': '0.875rem' }}>Loading...</p>
        </Show>

        <Show when={error()}>
          <p style={{ color: 'var(--accent-red)', 'font-size': '0.875rem' }}>{error()}</p>
        </Show>

        <Show when={status()}>
          <div style={{ display: 'flex', 'flex-direction': 'column', gap: '0.75rem' }}>
            <div style={{ display: 'flex', 'align-items': 'center', gap: '0.625rem' }}>
              <span style={{
                width: '8px',
                height: '8px',
                'border-radius': '50%',
                background: status()!.running ? 'var(--accent-green)' : 'var(--accent-red)',
                'flex-shrink': '0',
              }} />
              <span style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>
                {status()!.running ? 'Running' : 'Stopped'}
              </span>
            </div>
            <Show when={status()!.running}>
              <div style={{ display: 'flex', gap: '2rem' }}>
                <div>
                  <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>API Port</div>
                  <div style={{ 'font-size': '0.875rem', 'font-family': 'monospace' }}>{status()!.api_port}</div>
                </div>
                <div>
                  <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>Gateway Port</div>
                  <div style={{ 'font-size': '0.875rem', 'font-family': 'monospace' }}>{status()!.gateway_port}</div>
                </div>
              </div>
            </Show>
            <Show when={status()!.node_id}>
              <div>
                <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)', 'margin-bottom': '0.25rem' }}>Node ID</div>
                <div style={{
                  'font-size': '0.75rem',
                  'font-family': 'monospace',
                  'word-break': 'break-all',
                  background: 'var(--bg)',
                  border: '1px solid var(--border)',
                  'border-radius': '6px',
                  padding: '0.5rem 0.75rem',
                }}>
                  {status()!.node_id}
                </div>
              </div>
            </Show>
          </div>
        </Show>
      </div>
    </div>
  );
};

function sectionHeaderStyle(): Record<string, string> {
  return {
    'font-size': '0.75rem',
    'font-weight': '600',
    'text-transform': 'uppercase',
    'letter-spacing': '0.05em',
    color: 'var(--muted-fg)',
    'margin-bottom': '1rem',
  };
}

export default Home;
