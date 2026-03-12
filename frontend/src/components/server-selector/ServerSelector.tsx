import React from 'react';
import { useServer } from '../../contexts/ServerContext';
import type { TunnelStatus } from '../../../../shared/protocol';

const statusColors: Record<TunnelStatus | string, string> = {
  connected: 'bg-green-500',
  connecting: 'bg-yellow-500',
  disconnected: 'bg-gray-400',
  error: 'bg-red-500',
};

const statusLabels: Record<TunnelStatus | string, string> = {
  connected: 'Connected',
  connecting: 'Connecting...',
  disconnected: 'Offline',
  error: 'Error',
};

export default function ServerSelector() {
  const { servers, selectedServerId, setSelectedServerId, serverStatuses } = useServer();

  if (servers.length <= 1) {
    // Only local server, no need to show selector
    return null;
  }

  return (
    <div className="flex items-center gap-2">
      <label className="text-xs text-gray-500 dark:text-gray-400">Server:</label>
      <select
        value={selectedServerId}
        onChange={(e) => setSelectedServerId(e.target.value)}
        className="text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded px-2 py-1 focus:outline-none focus:ring-2 focus:ring-blue-500"
      >
        {servers.map((server) => {
          const status = server.isLocal ? 'connected' : (serverStatuses[server.id] || 'disconnected');
          return (
            <option key={server.id} value={server.id}>
              {server.name} ({statusLabels[status] || status})
            </option>
          );
        })}
      </select>
      <StatusDot status={
        selectedServerId === 'local'
          ? 'connected'
          : (serverStatuses[selectedServerId] || 'disconnected')
      } />
    </div>
  );
}

function StatusDot({ status }: { status: string }) {
  const color = statusColors[status] || statusColors.disconnected;
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${color}`}
      title={statusLabels[status] || status}
    />
  );
}
