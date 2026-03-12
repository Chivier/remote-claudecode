import React, { useState } from 'react';
import { useServer } from '../../../../../contexts/ServerContext';
import RemoteServerForm from './RemoteServerForm';
import type { Server } from '../../../../../../../shared/protocol';

export default function RemoteServersSettingsTab() {
  const {
    servers,
    deleteServer,
    testConnection,
    deployBroker,
    serverStatuses,
    refreshServers,
  } = useServer();

  const [showForm, setShowForm] = useState(false);
  const [editingServer, setEditingServer] = useState<Server | null>(null);
  const [testResults, setTestResults] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState<Record<string, boolean>>({});

  const handleTest = async (id: string) => {
    setLoading(prev => ({ ...prev, [id]: true }));
    try {
      const result = await testConnection(id);
      setTestResults(prev => ({
        ...prev,
        [id]: result.success ? `OK: ${result.output}` : `Error: ${result.error}`,
      }));
    } catch (e: any) {
      setTestResults(prev => ({ ...prev, [id]: `Error: ${e.message}` }));
    }
    setLoading(prev => ({ ...prev, [id]: false }));
  };

  const handleDeploy = async (id: string) => {
    setLoading(prev => ({ ...prev, [`deploy-${id}`]: true }));
    try {
      const result = await deployBroker(id);
      setTestResults(prev => ({
        ...prev,
        [`deploy-${id}`]: result.success ? `OK: ${result.output}` : `Error: ${result.error}`,
      }));
    } catch (e: any) {
      setTestResults(prev => ({ ...prev, [`deploy-${id}`]: `Error: ${e.message}` }));
    }
    setLoading(prev => ({ ...prev, [`deploy-${id}`]: false }));
  };

  const handleDelete = async (id: string) => {
    if (!window.confirm('Are you sure you want to delete this server?')) return;
    try {
      await deleteServer(id);
    } catch (e: any) {
      alert(e.message);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-lg font-semibold">Remote Servers</h2>
        <button
          onClick={() => { setEditingServer(null); setShowForm(true); }}
          className="px-3 py-1 bg-blue-600 text-white rounded text-sm hover:bg-blue-700"
        >
          Add Server
        </button>
      </div>

      {showForm && (
        <RemoteServerForm
          server={editingServer}
          onClose={() => { setShowForm(false); setEditingServer(null); }}
          onSaved={() => { setShowForm(false); setEditingServer(null); refreshServers(); }}
        />
      )}

      <div className="space-y-3">
        {servers.map((server) => {
          const status = server.isLocal ? 'connected' : (serverStatuses[server.id] || 'disconnected');
          return (
            <div
              key={server.id}
              className="border border-gray-200 dark:border-gray-700 rounded-lg p-4"
            >
              <div className="flex justify-between items-start">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="font-medium">{server.name}</h3>
                    {server.isLocal && (
                      <span className="text-xs bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded">
                        Local
                      </span>
                    )}
                    <span
                      className={`inline-block w-2 h-2 rounded-full ${
                        status === 'connected' ? 'bg-green-500' :
                        status === 'connecting' ? 'bg-yellow-500' :
                        status === 'error' ? 'bg-red-500' : 'bg-gray-400'
                      }`}
                    />
                  </div>
                  {!server.isLocal && (
                    <p className="text-sm text-gray-500 mt-1">
                      {server.sshUser ? `${server.sshUser}@` : ''}{server.hostname}:{server.sshPort}
                      {' '}(Broker port: {server.brokerPort})
                    </p>
                  )}
                </div>

                {!server.isLocal && (
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleTest(server.id)}
                      disabled={loading[server.id]}
                      className="text-xs px-2 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                      {loading[server.id] ? 'Testing...' : 'Test'}
                    </button>
                    <button
                      onClick={() => handleDeploy(server.id)}
                      disabled={loading[`deploy-${server.id}`]}
                      className="text-xs px-2 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                      {loading[`deploy-${server.id}`] ? 'Deploying...' : 'Deploy'}
                    </button>
                    <button
                      onClick={() => { setEditingServer(server); setShowForm(true); }}
                      className="text-xs px-2 py-1 border rounded hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(server.id)}
                      className="text-xs px-2 py-1 border border-red-300 text-red-600 rounded hover:bg-red-50"
                    >
                      Delete
                    </button>
                  </div>
                )}
              </div>

              {testResults[server.id] && (
                <p className="text-xs mt-2 text-gray-600 dark:text-gray-400 font-mono">
                  {testResults[server.id]}
                </p>
              )}
              {testResults[`deploy-${server.id}`] && (
                <p className="text-xs mt-2 text-gray-600 dark:text-gray-400 font-mono">
                  {testResults[`deploy-${server.id}`]}
                </p>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
