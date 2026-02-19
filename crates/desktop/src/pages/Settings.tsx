import { Component, createSignal, onMount, Show } from 'solid-js';
import { getConfigInfo, ConfigInfo } from '../lib/api';

type ThemeOption = 'system' | 'light' | 'dark';
type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'error' | 'up-to-date';

const Settings: Component = () => {
  // Auto-launch state
  const [autoLaunch, setAutoLaunch] = createSignal(false);
  const [autoLaunchLoading, setAutoLaunchLoading] = createSignal(true);

  // Theme state
  const [theme, setTheme] = createSignal<ThemeOption>('system');

  // Config info
  const [configInfo, setConfigInfo] = createSignal<ConfigInfo | null>(null);
  const [configLoading, setConfigLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // Update state
  const [updateStatus, setUpdateStatus] = createSignal<UpdateStatus>('idle');
  const [updateVersion, setUpdateVersion] = createSignal<string | null>(null);
  const [updateError, setUpdateError] = createSignal<string | null>(null);

  onMount(async () => {
    // Load auto-launch state
    try {
      const { isEnabled } = await import('@tauri-apps/plugin-autostart');
      const enabled = await isEnabled();
      setAutoLaunch(enabled);
    } catch (_e) {
      // Plugin may not be available
    } finally {
      setAutoLaunchLoading(false);
    }

    // Load theme from localStorage
    const saved = localStorage.getItem('jax-theme') as ThemeOption | null;
    if (saved === 'light' || saved === 'dark') {
      setTheme(saved);
    }

    // Load config info
    try {
      const info = await getConfigInfo();
      setConfigInfo(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setConfigLoading(false);
    }
  });

  const toggleAutoLaunch = async () => {
    try {
      if (autoLaunch()) {
        const { disable } = await import('@tauri-apps/plugin-autostart');
        await disable();
        setAutoLaunch(false);
      } else {
        const { enable } = await import('@tauri-apps/plugin-autostart');
        await enable();
        setAutoLaunch(true);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const checkForUpdate = async () => {
    setUpdateStatus('checking');
    setUpdateError(null);
    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (update) {
        setUpdateVersion(update.version);
        setUpdateStatus('available');
      } else {
        setUpdateStatus('up-to-date');
      }
    } catch (e) {
      setUpdateError(String(e));
      setUpdateStatus('error');
    }
  };

  const installUpdate = async () => {
    setUpdateStatus('downloading');
    setUpdateError(null);
    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        const { relaunch } = await import('@tauri-apps/plugin-process');
        await relaunch();
      }
    } catch (e) {
      setUpdateError(String(e));
      setUpdateStatus('error');
    }
  };

  const applyTheme = (value: ThemeOption) => {
    setTheme(value);
    if (value === 'system') {
      localStorage.removeItem('jax-theme');
      document.documentElement.removeAttribute('data-theme');
    } else {
      localStorage.setItem('jax-theme', value);
      document.documentElement.setAttribute('data-theme', value);
    }
  };

  return (
    <div style={{ 'max-width': '640px' }}>
      <h2 style={{ 'font-size': '1.5rem', 'font-weight': '700', 'margin-bottom': '1.5rem' }}>
        Settings
      </h2>

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

      {/* General */}
      <div style={cardStyle()}>
        <h3 style={sectionHeaderStyle()}>General</h3>

        {/* Auto-launch toggle */}
        <div style={settingRowStyle()}>
          <div>
            <div style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>Launch at Login</div>
            <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>
              Start Jax automatically when you log in
            </div>
          </div>
          <Show when={!autoLaunchLoading()}>
            <button
              onClick={toggleAutoLaunch}
              style={toggleStyle(autoLaunch())}
            >
              <span style={toggleKnobStyle(autoLaunch())} />
            </button>
          </Show>
        </div>
      </div>

      {/* Updates */}
      <div style={cardStyle()}>
        <h3 style={sectionHeaderStyle()}>Updates</h3>

        <Show when={updateError()}>
          <div style={{
            background: 'hsl(0 84% 60% / 0.08)',
            border: '1px solid hsl(0 84% 60% / 0.3)',
            padding: '0.5rem 0.75rem',
            'border-radius': '6px',
            'margin-bottom': '0.75rem',
            color: 'var(--accent-red)',
            'font-size': '0.8125rem',
          }}>
            {updateError()}
          </div>
        </Show>

        <div style={settingRowStyle()}>
          <div>
            <Show when={updateStatus() === 'idle' || updateStatus() === 'error'}>
              <div style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>Check for updates</div>
              <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>
                See if a newer version of Jax is available
              </div>
            </Show>
            <Show when={updateStatus() === 'checking'}>
              <div style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>Checking...</div>
            </Show>
            <Show when={updateStatus() === 'up-to-date'}>
              <div style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>Up to date</div>
              <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>
                You are running the latest version
              </div>
            </Show>
            <Show when={updateStatus() === 'available'}>
              <div style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>
                Update available: v{updateVersion()}
              </div>
              <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>
                A new version is ready to install
              </div>
            </Show>
            <Show when={updateStatus() === 'downloading'}>
              <div style={{ 'font-size': '0.875rem', 'font-weight': '500' }}>Downloading update...</div>
              <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)' }}>
                The app will restart when ready
              </div>
            </Show>
          </div>

          <Show when={updateStatus() === 'idle' || updateStatus() === 'up-to-date' || updateStatus() === 'error'}>
            <button
              onClick={checkForUpdate}
              style={{
                padding: '0.375rem 0.75rem',
                'border-radius': '6px',
                border: '1px solid var(--border)',
                background: 'var(--bg)',
                color: 'var(--fg)',
                cursor: 'pointer',
                'font-size': '0.8125rem',
                'font-family': 'inherit',
                'flex-shrink': '0',
              }}
            >
              Check
            </button>
          </Show>
          <Show when={updateStatus() === 'available'}>
            <button
              onClick={installUpdate}
              style={{
                padding: '0.375rem 0.75rem',
                'border-radius': '6px',
                border: '1px solid var(--accent-green)',
                background: 'var(--accent-green)',
                color: 'white',
                cursor: 'pointer',
                'font-size': '0.8125rem',
                'font-weight': '600',
                'font-family': 'inherit',
                'flex-shrink': '0',
              }}
            >
              Install
            </button>
          </Show>
        </div>
      </div>

      {/* Appearance */}
      <div style={cardStyle()}>
        <h3 style={sectionHeaderStyle()}>Appearance</h3>

        <div style={{
          display: 'flex',
          gap: '0.5rem',
        }}>
          {(['system', 'light', 'dark'] as ThemeOption[]).map(opt => (
            <button
              onClick={() => applyTheme(opt)}
              style={{
                flex: '1',
                padding: '0.5rem 0.75rem',
                'border-radius': '8px',
                border: '1px solid ' + (theme() === opt ? 'var(--fg)' : 'var(--border)'),
                background: theme() === opt ? 'var(--fg)' : 'var(--bg)',
                color: theme() === opt ? 'var(--bg)' : 'var(--fg)',
                cursor: 'pointer',
                'font-size': '0.8125rem',
                'font-weight': theme() === opt ? '600' : '400',
                'font-family': 'inherit',
                'text-transform': 'capitalize',
              }}
            >
              {opt}
            </button>
          ))}
        </div>
      </div>

      {/* Local Configuration */}
      <div style={cardStyle()}>
        <h3 style={sectionHeaderStyle()}>Local Configuration</h3>

        <Show when={configLoading()}>
          <p style={{ color: 'var(--muted-fg)', 'font-size': '0.875rem' }}>Loading...</p>
        </Show>

        <Show when={configInfo()}>
          <div style={{ display: 'flex', 'flex-direction': 'column', gap: '0.75rem' }}>
            <ConfigRow label="Data directory" value={configInfo()!.jax_dir} />
            <ConfigRow label="Database" value={configInfo()!.db_path} />
            <ConfigRow label="Config file" value={configInfo()!.config_path} />
            <ConfigRow label="Blob store" value={configInfo()!.blob_store} />
          </div>
        </Show>
      </div>
    </div>
  );
};

const ConfigRow: Component<{ label: string; value: string }> = (props) => (
  <div>
    <div style={{ 'font-size': '0.75rem', color: 'var(--muted-fg)', 'margin-bottom': '0.125rem' }}>
      {props.label}
    </div>
    <div style={{
      'font-size': '0.75rem',
      'font-family': 'monospace',
      'word-break': 'break-all',
      background: 'var(--bg)',
      border: '1px solid var(--border)',
      'border-radius': '6px',
      padding: '0.375rem 0.625rem',
    }}>
      {props.value}
    </div>
  </div>
);

function cardStyle(): Record<string, string> {
  return {
    background: 'var(--muted)',
    border: '1px solid var(--border)',
    'border-radius': 'var(--radius)',
    padding: '1.5rem',
    'margin-bottom': '1rem',
  };
}

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

function settingRowStyle(): Record<string, string> {
  return {
    display: 'flex',
    'justify-content': 'space-between',
    'align-items': 'center',
  };
}

function toggleStyle(on: boolean): Record<string, string> {
  return {
    width: '44px',
    height: '24px',
    'border-radius': '12px',
    border: 'none',
    background: on ? 'var(--accent-green)' : 'var(--border)',
    cursor: 'pointer',
    position: 'relative',
    transition: 'background 0.2s ease',
    'flex-shrink': '0',
  };
}

function toggleKnobStyle(on: boolean): Record<string, string> {
  return {
    position: 'absolute',
    top: '2px',
    left: on ? '22px' : '2px',
    width: '20px',
    height: '20px',
    'border-radius': '50%',
    background: 'white',
    transition: 'left 0.2s ease',
    'box-shadow': '0 1px 3px rgba(0,0,0,0.2)',
  };
}

export default Settings;
