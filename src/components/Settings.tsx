import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Save, Download, Upload, RefreshCw } from 'lucide-react';

interface UserSettings {
  autoStart: boolean;
  minimizeToTray: boolean;
  checkUpdates: boolean;
  theme: 'light' | 'dark' | 'system';
  refreshInterval: number;
  backupLocation: string;
  backupFrequency: 'daily' | 'weekly' | 'monthly';
  logLevel: 'error' | 'warn' | 'info' | 'debug';
  enableLogs: boolean;
  developerMode: boolean;
  sourceOfTruth: string;
  autoSync: boolean;
  enabledApps: {
    'Claude Desktop': boolean;
    'Cursor': boolean;
    'Amazon Q Developer': boolean;
    'Visual Studio Code': boolean;
    'Warp': boolean;
    'Claude Code': boolean;
    'Zed': boolean;
    'Continue.dev': boolean;
    'IntelliJ IDEA': boolean;
    'PHPStorm': boolean;
    'WebStorm': boolean;
    'PyCharm': boolean;
  };
}

const defaultSettings: UserSettings = {
  autoStart: false,
  minimizeToTray: true,
  checkUpdates: true,
  theme: 'system',
  refreshInterval: 10,
  backupLocation: '',
  backupFrequency: 'weekly',
  logLevel: 'info',
  enableLogs: true,
  developerMode: false,
  sourceOfTruth: 'none',
  autoSync: false,
  enabledApps: {
    'Claude Desktop': true,
    'Cursor': true,
    'Amazon Q Developer': true,
    'Visual Studio Code': true,
    'Warp': true,
    'Claude Code': true,
    'Zed': false,
    'Continue.dev': false,
    'IntelliJ IDEA': false,
    'PHPStorm': false,
    'WebStorm': false,
    'PyCharm': false,
  },
};

interface SettingsProps {
  onSettingsSaved?: () => void;
}

export default function Settings({ onSettingsSaved }: SettingsProps) {
  const [settings, setSettings] = useState<UserSettings>(defaultSettings);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const savedSettings = await invoke<UserSettings>('get_settings');
      setSettings({ ...defaultSettings, ...savedSettings });
    } catch (error) {
      console.error('Failed to load settings:', error);
      setSettings(defaultSettings);
    } finally {
      setLoading(false);
    }
  };

  const saveSettings = async () => {
    setSaving(true);
    setMessage(null);
    try {
      await invoke('save_settings', { settings });
      
      // Apply theme immediately
      applyTheme(settings.theme);
      
      setMessage('Settings saved successfully!');
      setTimeout(() => setMessage(null), 3000);
      
      // Refresh data if callback provided (for app filtering changes)
      if (onSettingsSaved) {
        onSettingsSaved();
      }
    } catch (error) {
      console.error('Failed to save settings:', error);
      setMessage('Failed to save settings');
    } finally {
      setSaving(false);
    }
  };

  const applyTheme = (theme: string) => {
    const root = document.documentElement;
    if (theme === 'dark') {
      root.classList.add('dark-theme');
      root.classList.remove('light-theme');
    } else if (theme === 'light') {
      root.classList.add('light-theme');
      root.classList.remove('dark-theme');
    } else {
      // System theme
      root.classList.remove('dark-theme', 'light-theme');
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      if (prefersDark) {
        root.classList.add('dark-theme');
      } else {
        root.classList.add('light-theme');
      }
    }
  };

  useEffect(() => {
    // Apply theme when settings load
    if (!loading) {
      applyTheme(settings.theme);
    }
  }, [settings.theme, loading]);

  const handleBackup = async () => {
    try {
      await invoke('create_backup');
      setMessage('Backup created successfully!');
      setTimeout(() => setMessage(null), 3000);
    } catch (error) {
      console.error('Failed to create backup:', error);
      setMessage('Failed to create backup');
    }
  };

  const handleExport = async () => {
    try {
      await invoke('export_config');
      setMessage('Configuration exported successfully!');
      setTimeout(() => setMessage(null), 3000);
    } catch (error) {
      console.error('Failed to export config:', error);
      setMessage('Failed to export configuration');
    }
  };

  const handleImport = async () => {
    try {
      await invoke('import_config');
      setMessage('Configuration imported successfully!');
      setTimeout(() => setMessage(null), 3000);
      loadSettings(); // Reload settings after import
    } catch (error) {
      console.error('Failed to import config:', error);
      setMessage('Failed to import configuration');
    }
  };

  const handleSync = async () => {
    if (settings.sourceOfTruth === 'none') {
      setMessage('Please select a source of truth first');
      setTimeout(() => setMessage(null), 3000);
      return;
    }

    setSyncing(true);
    setMessage(null);
    try {
      const result = await invoke<string>('sync_from_source', {
        sourceApp: settings.sourceOfTruth
      });
      setMessage(result);
      setTimeout(() => setMessage(null), 5000);

      // Refresh data if callback provided
      if (onSettingsSaved) {
        onSettingsSaved();
      }
    } catch (error) {
      console.error('Failed to sync:', error);
      setMessage(`Failed to sync: ${error}`);
    } finally {
      setSyncing(false);
    }
  };

  if (loading) {
    return (
      <div className="loading">
        <RefreshCw size={24} style={{ animation: 'spin 1s linear infinite' }} />
        <p style={{ marginTop: '10px' }}>Loading settings...</p>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
        <h2>Settings</h2>
        <button 
          className="btn btn-primary" 
          onClick={saveSettings}
          disabled={saving}
        >
          <Save size={16} style={{ marginRight: '8px' }} />
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
      </div>

      {message && (
        <div style={{ 
          padding: '12px', 
          marginBottom: '20px',
          borderRadius: '6px',
          background: message.includes('Failed') ? '#ffebee' : '#e8f5e8',
          color: message.includes('Failed') ? '#c62828' : '#2e7d32',
          border: `1px solid ${message.includes('Failed') ? '#ffcdd2' : '#c8e6c8'}`
        }}>
          {message}
        </div>
      )}

      <div style={{ display: 'grid', gap: '20px' }}>
        {/* General Settings */}
        <div style={{ background: 'var(--bg-secondary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
          <h3 style={{ marginBottom: '16px', color: 'var(--text-primary)' }}>General Settings</h3>
          
          <div style={{ display: 'grid', gap: '12px' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
              <input
                type="checkbox"
                checked={settings.autoStart}
                onChange={(e) => setSettings({ ...settings, autoStart: e.target.checked })}
              />
              Auto-start on login
            </label>
            
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
              <input
                type="checkbox"
                checked={settings.minimizeToTray}
                onChange={(e) => setSettings({ ...settings, minimizeToTray: e.target.checked })}
              />
              Minimize to system tray
            </label>
            
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
              <input
                type="checkbox"
                checked={settings.enableLogs}
                onChange={(e) => setSettings({ ...settings, enableLogs: e.target.checked })}
              />
              Enable logging
            </label>
            
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-secondary)' }}>
              <input
                type="checkbox"
                checked={false}
                disabled={true}
                style={{ opacity: 0.5 }}
              />
              <span style={{ textDecoration: 'line-through' }}>Check for updates automatically</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
                (Coming Soon)
              </span>
            </label>
          </div>
        </div>

        {/* Theme Settings */}
        <div style={{ background: 'var(--bg-secondary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
          <h3 style={{ marginBottom: '16px', color: 'var(--text-primary)' }}>Theme Settings</h3>
          
          <div style={{ display: 'grid', gap: '12px' }}>
            <label style={{ color: 'var(--text-primary)' }}>
              Theme:
              <select
                value={settings.theme}
                onChange={(e) => setSettings({ ...settings, theme: e.target.value as any })}
                style={{ 
                  marginLeft: '8px', 
                  padding: '4px 8px',
                  background: 'var(--bg-primary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px'
                }}
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">System</option>
              </select>
            </label>
            
            <label style={{ color: 'var(--text-primary)' }}>
              Refresh interval (seconds):
              <input
                type="number"
                min="5"
                max="300"
                value={settings.refreshInterval}
                onChange={(e) => setSettings({ ...settings, refreshInterval: parseInt(e.target.value) })}
                style={{ 
                  marginLeft: '8px', 
                  padding: '4px 8px', 
                  width: '80px',
                  background: 'var(--bg-primary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px'
                }}
              />
            </label>
          </div>
        </div>

        {/* Source of Truth Configuration */}
        <div style={{ background: 'var(--bg-secondary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
          <h3 style={{ marginBottom: '16px', color: 'var(--text-primary)' }}>Source of Truth Configuration</h3>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '16px', fontSize: '14px' }}>
            Select which application should be the authoritative source for MCP server configurations.
            When syncing, all other applications will be updated to match the source.
          </p>

          <div style={{ display: 'grid', gap: '12px' }}>
            <label style={{ color: 'var(--text-primary)' }}>
              Source of Truth:
              <select
                value={settings.sourceOfTruth}
                onChange={(e) => setSettings({ ...settings, sourceOfTruth: e.target.value })}
                style={{
                  marginLeft: '8px',
                  padding: '6px 12px',
                  background: 'var(--bg-primary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  minWidth: '200px'
                }}
              >
                <option value="none">None (Manual Sync Only)</option>
                <option value="MCP Control Lite">MCP Control Lite</option>
                <option value="Claude Desktop">Claude Desktop</option>
                <option value="Claude Code">Claude Code</option>
                <option value="Cursor">Cursor</option>
                <option value="Warp">Warp</option>
                <option value="Visual Studio Code">Visual Studio Code</option>
                <option value="Amazon Q Developer">Amazon Q Developer</option>
                <option value="IntelliJ IDEA">IntelliJ IDEA</option>
                <option value="PHPStorm">PHPStorm</option>
                <option value="WebStorm">WebStorm</option>
                <option value="PyCharm">PyCharm</option>
                <option value="Zed">Zed</option>
                <option value="Continue.dev">Continue.dev</option>
              </select>
            </label>

            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-secondary)' }}>
              <input
                type="checkbox"
                checked={false}
                disabled={true}
                style={{ opacity: 0.5 }}
              />
              <span style={{ textDecoration: 'line-through' }}>Auto-sync when source changes</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
                (Coming Soon)
              </span>
            </label>

            <div style={{ marginTop: '10px' }}>
              <button
                className="btn btn-primary"
                onClick={handleSync}
                disabled={syncing || settings.sourceOfTruth === 'none'}
                style={{ opacity: settings.sourceOfTruth === 'none' ? 0.5 : 1 }}
              >
                <RefreshCw size={16} style={{ marginRight: '8px' }} />
                {syncing ? 'Syncing...' : 'Sync Now'}
              </button>
              {settings.sourceOfTruth !== 'none' && (
                <p style={{ color: 'var(--text-secondary)', marginTop: '8px', fontSize: '13px' }}>
                  This will copy all MCP servers from "{settings.sourceOfTruth}" to all other detected applications.
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Backup & Restore */}
        <div style={{ background: 'var(--bg-secondary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
          <h3 style={{ marginBottom: '16px', color: 'var(--text-primary)' }}>Backup & Restore</h3>
          
          <div style={{ display: 'grid', gap: '12px' }}>
            <label style={{ color: 'var(--text-primary)' }}>
              Backup frequency:
              <select
                value={settings.backupFrequency}
                onChange={(e) => setSettings({ ...settings, backupFrequency: e.target.value as any })}
                style={{ 
                  marginLeft: '8px', 
                  padding: '4px 8px',
                  background: 'var(--bg-primary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px'
                }}
              >
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
                <option value="monthly">Monthly</option>
              </select>
            </label>
            
            <div style={{ display: 'flex', gap: '10px', marginTop: '10px' }}>
              <button className="btn btn-secondary" onClick={handleBackup}>
                <Save size={16} style={{ marginRight: '8px' }} />
                Create Backup
              </button>
              <button className="btn btn-secondary" onClick={handleExport}>
                <Download size={16} style={{ marginRight: '8px' }} />
                Export Config
              </button>
              <button className="btn btn-secondary" onClick={handleImport}>
                <Upload size={16} style={{ marginRight: '8px' }} />
                Import Config
              </button>
            </div>
          </div>
        </div>

        {/* Application Filtering */}
        <div style={{ background: 'var(--bg-secondary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
          <h3 style={{ marginBottom: '16px', color: 'var(--text-primary)' }}>Application Filtering</h3>
          <p style={{ color: 'var(--text-secondary)', marginBottom: '16px', fontSize: '14px' }}>
            Enable or disable applications to show only the ones you use. Disabled applications won't appear in the dashboard.
          </p>
          
          <div style={{ display: 'grid', gap: '8px' }}>
            {Object.entries(settings.enabledApps).map(([app, enabled]) => (
              <label key={app} style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(e) => setSettings({ 
                    ...settings, 
                    enabledApps: { ...settings.enabledApps, [app]: e.target.checked }
                  })}
                />
                {app}
              </label>
            ))}
          </div>
        </div>

        {/* Advanced Settings */}
        <div style={{ background: 'var(--bg-secondary)', padding: '20px', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
          <h3 style={{ marginBottom: '16px', color: 'var(--text-primary)' }}>Advanced Settings</h3>
          
          <div style={{ display: 'grid', gap: '12px' }}>
            <label style={{ color: 'var(--text-primary)' }}>
              Log level:
              <select
                value={settings.logLevel}
                onChange={(e) => setSettings({ ...settings, logLevel: e.target.value as any })}
                style={{ 
                  marginLeft: '8px', 
                  padding: '4px 8px',
                  background: 'var(--bg-primary)',
                  color: 'var(--text-primary)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px'
                }}
              >
                <option value="error">Error</option>
                <option value="warn">Warning</option>
                <option value="info">Info</option>
                <option value="debug">Debug</option>
              </select>
            </label>
            
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
              <input
                type="checkbox"
                checked={settings.developerMode}
                onChange={(e) => setSettings({ ...settings, developerMode: e.target.checked })}
              />
              Developer mode (show advanced options)
            </label>
          </div>
        </div>
      </div>
    </div>
  );
}
