import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Save, RotateCcw, X, Settings, Shield, Link, Monitor, Code } from 'lucide-react';

interface ServerConfig {
  name: string;
  description: string;
  enabled: boolean;
  command: string;
  args: string[];
  env: Record<string, string>;
  port?: number;
  host?: string;
  protocol: 'http' | 'https' | 'tcp';
  tlsEnabled: boolean;
  tlsCertPath?: string;
  tlsKeyPath?: string;
  authEnabled: boolean;
  authToken?: string;
  rateLimit?: number;
  timeout?: number;
  dependencies: string[];
  startupOrder: number;
  restartOnFailure: boolean;
}

interface ServerDetailProps {
  serverId: string;
  serverName: string;
  application: string;
  onClose: () => void;
  onSave: () => void;
  isNewServer?: boolean;
}

export default function ServerDetail({ serverId, serverName, application, onClose, onSave, isNewServer = false }: ServerDetailProps) {
  const [config, setConfig] = useState<ServerConfig>({
    name: serverName,
    description: '',
    enabled: true,
    command: '',
    args: [],
    env: {},
    protocol: 'http',
    tlsEnabled: false,
    authEnabled: false,
    dependencies: [],
    startupOrder: 0,
    restartOnFailure: true
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<'basic' | 'advanced' | 'dependencies' | 'applications' | 'editor'>('basic');
  const [message, setMessage] = useState<string | null>(null);
  const [jsonEditor, setJsonEditor] = useState<string>('');

  useEffect(() => {
    if (!isNewServer) {
      loadServerConfig();
    } else {
      setLoading(false);
    }
  }, [serverId, isNewServer]);

  useEffect(() => {
    if (activeTab === 'editor') {
      updateJsonEditor();
    }
  }, [activeTab, config]);

  const loadServerConfig = async () => {
    try {
      const serverConfig = await invoke<ServerConfig>('get_server_config', { 
        serverId, 
        application 
      });
      // Ensure env is always an object
      setConfig({
        ...serverConfig,
        env: serverConfig.env || {}
      });
    } catch (error) {
      console.error('Failed to load server config:', error);
      setMessage('Failed to load server configuration');
    } finally {
      setLoading(false);
    }
  };

  const saveConfig = async () => {
    setSaving(true);
    setMessage(null);
    try {
      if (isNewServer) {
        await invoke('create_server', { 
          application, 
          config 
        });
        setMessage('Server created successfully!');
      } else {
        await invoke('save_server_config', { 
          serverId, 
          application, 
          config 
        });
        setMessage('Configuration saved successfully!');
      }
      setTimeout(() => setMessage(null), 3000);
      onSave();
    } catch (error) {
      console.error('Failed to save config:', error);
      setMessage(isNewServer ? 'Failed to create server' : 'Failed to save configuration');
    } finally {
      setSaving(false);
    }
  };

  const resetConfig = async () => {
    try {
      await loadServerConfig();
      setMessage('Configuration reset to saved values');
      setTimeout(() => setMessage(null), 3000);
    } catch (error) {
      console.error('Failed to reset config:', error);
    }
  };

  const addArg = () => {
    setConfig({ ...config, args: [...config.args, ''] });
  };

  const updateArg = (index: number, value: string) => {
    const newArgs = [...config.args];
    newArgs[index] = value;
    setConfig({ ...config, args: newArgs });
  };

  const removeArg = (index: number) => {
    setConfig({ ...config, args: config.args.filter((_, i) => i !== index) });
  };

  const addEnv = () => {
    const newKey = `ENV_VAR_${Object.keys(config.env).length + 1}`;
    setConfig({ ...config, env: { ...config.env, [newKey]: '' } });
  };

  const updateEnvKey = (oldKey: string, newKey: string) => {
    if (oldKey === newKey) return;
    const newEnv = { ...config.env };
    const value = newEnv[oldKey];
    delete newEnv[oldKey];
    newEnv[newKey] = value;
    setConfig({ ...config, env: newEnv });
  };

  const updateEnvValue = (key: string, value: string) => {
    setConfig({ ...config, env: { ...config.env, [key]: value } });
  };

  const removeEnv = (key: string) => {
    const newEnv = { ...config.env };
    delete newEnv[key];
    setConfig({ ...config, env: newEnv });
  };

  const updateJsonEditor = () => {
    setJsonEditor(JSON.stringify(config, null, 2));
  };

  const applyJsonEditor = () => {
    try {
      const parsedConfig = JSON.parse(jsonEditor);
      setConfig({ ...config, ...parsedConfig, env: parsedConfig.env || {} });
      setMessage('JSON configuration applied successfully!');
      setTimeout(() => setMessage(null), 3000);
    } catch (error) {
      setMessage('Invalid JSON format. Please check your syntax.');
      setTimeout(() => setMessage(null), 3000);
    }
  };

  if (loading) {
    return (
      <div style={{ 
        position: 'fixed', 
        top: 0, 
        left: 0, 
        right: 0, 
        bottom: 0, 
        background: 'rgba(0,0,0,0.5)', 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center',
        zIndex: 1000
      }}>
        <div style={{ 
          background: 'var(--bg-secondary)', 
          padding: '40px', 
          borderRadius: '8px',
          color: 'var(--text-primary)'
        }}>
          Loading server configuration...
        </div>
      </div>
    );
  }

  return (
    <div style={{ 
      position: 'fixed', 
      top: 0, 
      left: 0, 
      right: 0, 
      bottom: 0, 
      background: 'rgba(0,0,0,0.5)', 
      display: 'flex', 
      alignItems: 'center', 
      justifyContent: 'center',
      zIndex: 1000
    }}>
      <div style={{ 
        background: 'var(--bg-secondary)', 
        width: '90%', 
        maxWidth: '800px', 
        height: '90%', 
        borderRadius: '8px',
        display: 'flex',
        flexDirection: 'column',
        border: '1px solid var(--border-color)'
      }}>
        {/* Header */}
        <div style={{ 
          padding: '20px', 
          borderBottom: '1px solid var(--border-color)',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}>
          <div>
            <h2 style={{ color: 'var(--text-primary)', margin: 0 }}>Server Configuration</h2>
            <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0 0' }}>
              {serverName} • {application}
            </p>
          </div>
          <button 
            onClick={onClose}
            style={{ 
              background: 'none', 
              border: 'none', 
              cursor: 'pointer',
              color: 'var(--text-secondary)'
            }}
          >
            <X size={24} />
          </button>
        </div>

        {message && (
          <div style={{ 
            padding: '12px 20px',
            background: message.includes('Failed') ? '#ffebee' : '#e8f5e8',
            color: message.includes('Failed') ? '#c62828' : '#2e7d32',
            borderBottom: '1px solid var(--border-color)'
          }}>
            {message}
          </div>
        )}

        {/* Tabs */}
        <div style={{ 
          display: 'flex', 
          borderBottom: '1px solid var(--border-color)',
          background: 'var(--bg-primary)'
        }}>
          {[
            { id: 'basic', label: 'Basic', icon: Settings },
            { id: 'advanced', label: 'Advanced', icon: Shield },
            { id: 'dependencies', label: 'Dependencies', icon: Link },
            { id: 'applications', label: 'Applications', icon: Monitor },
            { id: 'editor', label: 'Code Editor', icon: Code }
          ].map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              onClick={() => setActiveTab(id as any)}
              style={{
                padding: '12px 20px',
                border: 'none',
                background: activeTab === id ? 'var(--bg-secondary)' : 'transparent',
                color: activeTab === id ? 'var(--text-primary)' : 'var(--text-secondary)',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                borderBottom: activeTab === id ? '2px solid #3498db' : '2px solid transparent'
              }}
            >
              <Icon size={16} />
              {label}
            </button>
          ))}
        </div>

        {/* Content */}
        <div style={{ flex: 1, padding: '20px', overflow: 'auto' }}>
          {/* Clarification Banner */}
          <div style={{
            background: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: '6px',
            padding: '12px 16px',
            marginBottom: '20px',
            color: 'var(--text-secondary)',
            fontSize: '14px'
          }}>
            <strong style={{ color: 'var(--text-primary)' }}>Note:</strong> This editor modifies MCP Control Lite's internal settings for this server, not the original MCP configuration files. Changes here affect how this application manages and displays the server.
          </div>

          {activeTab === 'basic' && (
            <div style={{ display: 'grid', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                  Server Name
                </label>
                <input
                  type="text"
                  value={config.name}
                  onChange={(e) => setConfig({ ...config, name: e.target.value })}
                  style={{
                    width: '100%',
                    padding: '8px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '4px',
                    background: 'var(--bg-primary)',
                    color: 'var(--text-primary)'
                  }}
                />
              </div>

              <div>
                <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                  Description
                </label>
                <textarea
                  value={config.description}
                  onChange={(e) => setConfig({ ...config, description: e.target.value })}
                  rows={3}
                  style={{
                    width: '100%',
                    padding: '8px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '4px',
                    background: 'var(--bg-primary)',
                    color: 'var(--text-primary)',
                    resize: 'vertical'
                  }}
                />
              </div>

              <div>
                <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                  Command
                </label>
                <input
                  type="text"
                  value={config.command}
                  onChange={(e) => setConfig({ ...config, command: e.target.value })}
                  style={{
                    width: '100%',
                    padding: '8px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '4px',
                    background: 'var(--bg-primary)',
                    color: 'var(--text-primary)'
                  }}
                />
              </div>

              <div>
                <label style={{ display: 'block', marginBottom: '8px', color: 'var(--text-primary)' }}>
                  Arguments
                </label>
                {config.args.map((arg, index) => (
                  <div key={index} style={{ display: 'flex', gap: '8px', marginBottom: '8px' }}>
                    <input
                      type="text"
                      value={arg}
                      onChange={(e) => updateArg(index, e.target.value)}
                      style={{
                        flex: 1,
                        padding: '8px',
                        border: '1px solid var(--border-color)',
                        borderRadius: '4px',
                        background: 'var(--bg-primary)',
                        color: 'var(--text-primary)'
                      }}
                    />
                    <button
                      onClick={() => removeArg(index)}
                      style={{
                        padding: '8px',
                        background: '#e74c3c',
                        color: 'white',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: 'pointer'
                      }}
                    >
                      <X size={16} />
                    </button>
                  </div>
                ))}
                <button
                  onClick={addArg}
                  className="btn btn-secondary"
                  style={{ marginTop: '8px' }}
                >
                  Add Argument
                </button>
              </div>

              <div>
                <label style={{ display: 'block', marginBottom: '8px', color: 'var(--text-primary)' }}>
                  Environment Variables
                </label>
                {Object.entries(config.env).map(([key, value], index) => (
                  <div key={index} style={{ display: 'flex', gap: '8px', marginBottom: '8px' }}>
                    <input
                      type="text"
                      placeholder="Key"
                      value={key}
                      onChange={(e) => updateEnvKey(key, e.target.value)}
                      style={{
                        flex: 1,
                        padding: '8px',
                        border: '1px solid var(--border-color)',
                        borderRadius: '4px',
                        background: 'var(--bg-primary)',
                        color: 'var(--text-primary)'
                      }}
                    />
                    <input
                      type="text"
                      placeholder="Value"
                      value={value}
                      onChange={(e) => updateEnvValue(key, e.target.value)}
                      style={{
                        flex: 2,
                        padding: '8px',
                        border: '1px solid var(--border-color)',
                        borderRadius: '4px',
                        background: 'var(--bg-primary)',
                        color: 'var(--text-primary)'
                      }}
                    />
                    <button
                      onClick={() => removeEnv(key)}
                      style={{
                        padding: '8px',
                        background: '#e74c3c',
                        color: 'white',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: 'pointer'
                      }}
                    >
                      <X size={16} />
                    </button>
                  </div>
                ))}
                <button
                  onClick={addEnv}
                  className="btn btn-secondary"
                  style={{ marginTop: '8px' }}
                >
                  Add Environment Variable
                </button>
              </div>
            </div>
          )}

          {activeTab === 'advanced' && (
            <div style={{ display: 'grid', gap: '16px' }}>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                <div>
                  <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                    Port
                  </label>
                  <input
                    type="number"
                    value={config.port || ''}
                    onChange={(e) => setConfig({ ...config, port: parseInt(e.target.value) || undefined })}
                    style={{
                      width: '100%',
                      padding: '8px',
                      border: '1px solid var(--border-color)',
                      borderRadius: '4px',
                      background: 'var(--bg-primary)',
                      color: 'var(--text-primary)'
                    }}
                  />
                </div>

                <div>
                  <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                    Protocol
                  </label>
                  <select
                    value={config.protocol}
                    onChange={(e) => setConfig({ ...config, protocol: e.target.value as any })}
                    style={{
                      width: '100%',
                      padding: '8px',
                      border: '1px solid var(--border-color)',
                      borderRadius: '4px',
                      background: 'var(--bg-primary)',
                      color: 'var(--text-primary)'
                    }}
                  >
                    <option value="http">HTTP</option>
                    <option value="https">HTTPS</option>
                    <option value="tcp">TCP</option>
                  </select>
                </div>
              </div>

              <div>
                <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
                  <input
                    type="checkbox"
                    checked={config.authEnabled}
                    onChange={(e) => setConfig({ ...config, authEnabled: e.target.checked })}
                  />
                  Enable Authentication
                </label>
              </div>

              {config.authEnabled && (
                <div>
                  <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                    Auth Token
                  </label>
                  <input
                    type="password"
                    value={config.authToken || ''}
                    onChange={(e) => setConfig({ ...config, authToken: e.target.value })}
                    style={{
                      width: '100%',
                      padding: '8px',
                      border: '1px solid var(--border-color)',
                      borderRadius: '4px',
                      background: 'var(--bg-primary)',
                      color: 'var(--text-primary)'
                    }}
                  />
                </div>
              )}

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
                <div>
                  <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                    Rate Limit (req/min)
                  </label>
                  <input
                    type="number"
                    value={config.rateLimit || ''}
                    onChange={(e) => setConfig({ ...config, rateLimit: parseInt(e.target.value) || undefined })}
                    style={{
                      width: '100%',
                      padding: '8px',
                      border: '1px solid var(--border-color)',
                      borderRadius: '4px',
                      background: 'var(--bg-primary)',
                      color: 'var(--text-primary)'
                    }}
                  />
                </div>

                <div>
                  <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                    Timeout (seconds)
                  </label>
                  <input
                    type="number"
                    value={config.timeout || ''}
                    onChange={(e) => setConfig({ ...config, timeout: parseInt(e.target.value) || undefined })}
                    style={{
                      width: '100%',
                      padding: '8px',
                      border: '1px solid var(--border-color)',
                      borderRadius: '4px',
                      background: 'var(--bg-primary)',
                      color: 'var(--text-primary)'
                    }}
                  />
                </div>
              </div>
            </div>
          )}

          {activeTab === 'applications' && (
            <div style={{ display: 'grid', gap: '16px' }}>
              <div>
                <h4 style={{ marginBottom: '12px', color: 'var(--text-primary)' }}>
                  Enable this server in applications:
                </h4>
                <div style={{ display: 'grid', gap: '8px' }}>
                  {['Claude Desktop', 'Cursor', 'Amazon Q Developer', 'Visual Studio Code', 'Zed'].map(app => (
                    <label key={app} style={{ 
                      display: 'flex', 
                      alignItems: 'center', 
                      gap: '8px',
                      padding: '8px 12px',
                      background: 'var(--bg-primary)',
                      borderRadius: '4px',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-primary)'
                    }}>
                      <input
                        type="checkbox"
                        defaultChecked={app === application}
                        onChange={(e) => {
                          // Handle app toggle
                          console.log(`Toggle ${config.name} in ${app}: ${e.target.checked}`);
                        }}
                      />
                      <div style={{ flex: 1 }}>
                        {app}
                        <div style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>
                          {app === application ? 'Currently configured' : 'Not configured'}
                        </div>
                      </div>
                    </label>
                  ))}
                </div>
              </div>
            </div>
          )}

          {activeTab === 'dependencies' && (
            <div style={{ display: 'grid', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', marginBottom: '4px', color: 'var(--text-primary)' }}>
                  Startup Order
                </label>
                <input
                  type="number"
                  value={config.startupOrder}
                  onChange={(e) => setConfig({ ...config, startupOrder: parseInt(e.target.value) || 0 })}
                  style={{
                    width: '100px',
                    padding: '8px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '4px',
                    background: 'var(--bg-primary)',
                    color: 'var(--text-primary)'
                  }}
                />
                <p style={{ fontSize: '12px', color: 'var(--text-secondary)', margin: '4px 0 0 0' }}>
                  Lower numbers start first
                </p>
              </div>

              <div>
                <label style={{ display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-primary)' }}>
                  <input
                    type="checkbox"
                    checked={config.restartOnFailure}
                    onChange={(e) => setConfig({ ...config, restartOnFailure: e.target.checked })}
                  />
                  Restart on failure
                </label>
              </div>
            </div>
          )}

          {activeTab === 'editor' && (
            <div style={{ display: 'grid', gap: '16px' }}>
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
                  <h4 style={{ color: 'var(--text-primary)', margin: 0 }}>JSON Configuration Editor</h4>
                  <div style={{ display: 'flex', gap: '8px' }}>
                    <button
                      onClick={updateJsonEditor}
                      className="btn btn-secondary"
                      style={{ fontSize: '12px', padding: '6px 12px' }}
                    >
                      Refresh from GUI
                    </button>
                    <button
                      onClick={applyJsonEditor}
                      className="btn btn-primary"
                      style={{ fontSize: '12px', padding: '6px 12px' }}
                    >
                      Apply to GUI
                    </button>
                  </div>
                </div>
                <textarea
                  value={jsonEditor}
                  onChange={(e) => setJsonEditor(e.target.value)}
                  style={{
                    width: '100%',
                    height: '400px',
                    padding: '12px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '4px',
                    background: 'var(--bg-primary)',
                    color: 'var(--text-primary)',
                    fontFamily: 'Monaco, Consolas, "Courier New", monospace',
                    fontSize: '12px',
                    lineHeight: '1.4',
                    resize: 'vertical'
                  }}
                  placeholder="JSON configuration will appear here..."
                />
                <p style={{ fontSize: '12px', color: 'var(--text-secondary)', margin: '8px 0 0 0' }}>
                  Edit the JSON configuration directly. Click "Apply to GUI" to update the form fields, or "Refresh from GUI" to sync from the current form values.
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{ 
          padding: '20px', 
          borderTop: '1px solid var(--border-color)',
          display: 'flex',
          justifyContent: 'flex-end',
          gap: '12px'
        }}>
          <button
            onClick={resetConfig}
            className="btn btn-secondary"
            disabled={saving}
          >
            <RotateCcw size={16} style={{ marginRight: '8px' }} />
            Reset
          </button>
          <button
            onClick={saveConfig}
            className="btn btn-primary"
            disabled={saving}
          >
            <Save size={16} style={{ marginRight: '8px' }} />
            {saving ? 'Saving...' : 'Save Configuration'}
          </button>
        </div>
      </div>
    </div>
  );
}
