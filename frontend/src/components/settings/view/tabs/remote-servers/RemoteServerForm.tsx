import React, { useState } from 'react';
import { useServer } from '../../../../../contexts/ServerContext';
import type { Server } from '../../../../../../../shared/protocol';

interface Props {
  server: Server | null;
  onClose: () => void;
  onSaved: () => void;
}

export default function RemoteServerForm({ server, onClose, onSaved }: Props) {
  const { addServer, updateServer } = useServer();
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [form, setForm] = useState({
    name: server?.name || '',
    hostname: server?.hostname || '',
    sshPort: server?.sshPort || 22,
    sshUser: server?.sshUser || '',
    sshKeyPath: server?.sshKeyPath || '',
    authMethod: server?.authMethod || 'key',
    brokerPort: server?.brokerPort || 9999,
    defaultWorkDir: server?.defaultWorkDir || '',
    autoUpdate: server?.autoUpdate ?? true,
    idleTimeoutSecs: server?.idleTimeoutSecs || 300,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);

    try {
      if (server) {
        await updateServer(server.id, form);
      } else {
        await addServer(form);
      }
      onSaved();
    } catch (e: any) {
      setError(e.message);
    }
    setSaving(false);
  };

  return (
    <form onSubmit={handleSubmit} className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 space-y-4">
      <h3 className="font-medium">{server ? 'Edit Server' : 'Add New Server'}</h3>

      {error && <p className="text-red-600 text-sm">{error}</p>}

      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium mb-1">Name</label>
          <input
            type="text"
            value={form.name}
            onChange={(e) => setForm(f => ({ ...f, name: e.target.value }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
            placeholder="My GPU Server"
            required
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Hostname</label>
          <input
            type="text"
            value={form.hostname}
            onChange={(e) => setForm(f => ({ ...f, hostname: e.target.value }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
            placeholder="192.168.1.100"
            required
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">SSH User</label>
          <input
            type="text"
            value={form.sshUser}
            onChange={(e) => setForm(f => ({ ...f, sshUser: e.target.value }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
            placeholder="root"
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">SSH Port</label>
          <input
            type="number"
            value={form.sshPort}
            onChange={(e) => setForm(f => ({ ...f, sshPort: parseInt(e.target.value) || 22 }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">SSH Key Path</label>
          <input
            type="text"
            value={form.sshKeyPath}
            onChange={(e) => setForm(f => ({ ...f, sshKeyPath: e.target.value }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
            placeholder="~/.ssh/id_rsa"
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Auth Method</label>
          <select
            value={form.authMethod}
            onChange={(e) => setForm(f => ({ ...f, authMethod: e.target.value }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
          >
            <option value="key">SSH Key</option>
            <option value="agent">SSH Agent</option>
            <option value="password">Password</option>
          </select>
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Broker Port</label>
          <input
            type="number"
            value={form.brokerPort}
            onChange={(e) => setForm(f => ({ ...f, brokerPort: parseInt(e.target.value) || 9999 }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Default Work Dir</label>
          <input
            type="text"
            value={form.defaultWorkDir}
            onChange={(e) => setForm(f => ({ ...f, defaultWorkDir: e.target.value }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
            placeholder="/home/user/projects"
          />
        </div>
        <div>
          <label className="block text-sm font-medium mb-1">Idle Timeout (seconds)</label>
          <input
            type="number"
            value={form.idleTimeoutSecs}
            onChange={(e) => setForm(f => ({ ...f, idleTimeoutSecs: parseInt(e.target.value) || 300 }))}
            className="w-full border rounded px-3 py-2 text-sm dark:bg-gray-800 dark:border-gray-600"
          />
        </div>
        <div className="flex items-center">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={form.autoUpdate}
              onChange={(e) => setForm(f => ({ ...f, autoUpdate: e.target.checked }))}
            />
            Auto-update Broker
          </label>
        </div>
      </div>

      <div className="flex gap-2 justify-end">
        <button
          type="button"
          onClick={onClose}
          className="px-3 py-1 border rounded text-sm hover:bg-gray-50 dark:hover:bg-gray-700"
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={saving}
          className="px-3 py-1 bg-blue-600 text-white rounded text-sm hover:bg-blue-700 disabled:opacity-50"
        >
          {saving ? 'Saving...' : (server ? 'Update' : 'Add Server')}
        </button>
      </div>
    </form>
  );
}
