import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import type { Server, TunnelStatus } from '../../../shared/protocol';

interface ServerContextType {
  servers: Server[];
  selectedServerId: string;
  selectedServer: Server | null;
  serverStatuses: Record<string, TunnelStatus>;
  setSelectedServerId: (id: string) => void;
  refreshServers: () => Promise<void>;
  addServer: (server: Partial<Server>) => Promise<Server>;
  updateServer: (id: string, updates: Partial<Server>) => Promise<Server>;
  deleteServer: (id: string) => Promise<void>;
  testConnection: (id: string) => Promise<{ success: boolean; output?: string; error?: string }>;
  deployBroker: (id: string) => Promise<{ success: boolean; output?: string; error?: string }>;
}

const ServerContext = createContext<ServerContextType | null>(null);

export function ServerProvider({ children }: { children: ReactNode }) {
  const [servers, setServers] = useState<Server[]>([]);
  const [selectedServerId, setSelectedServerId] = useState<string>('local');
  const [serverStatuses, setServerStatuses] = useState<Record<string, TunnelStatus>>({});

  const getAuthHeaders = useCallback(() => {
    const token = localStorage.getItem('token');
    return {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    };
  }, []);

  const refreshServers = useCallback(async () => {
    try {
      const res = await fetch('/api/remote-servers', { headers: getAuthHeaders() });
      if (res.ok) {
        const data = await res.json();
        setServers(data.servers || []);
      }
    } catch (e) {
      console.error('Failed to fetch servers:', e);
    }
  }, [getAuthHeaders]);

  const addServer = useCallback(async (server: Partial<Server>): Promise<Server> => {
    const res = await fetch('/api/remote-servers', {
      method: 'POST',
      headers: getAuthHeaders(),
      body: JSON.stringify(server),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error);
    await refreshServers();
    return data.server;
  }, [getAuthHeaders, refreshServers]);

  const updateServer = useCallback(async (id: string, updates: Partial<Server>): Promise<Server> => {
    const res = await fetch(`/api/remote-servers/${id}`, {
      method: 'PUT',
      headers: getAuthHeaders(),
      body: JSON.stringify(updates),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error);
    await refreshServers();
    return data.server;
  }, [getAuthHeaders, refreshServers]);

  const deleteServer = useCallback(async (id: string) => {
    const res = await fetch(`/api/remote-servers/${id}`, {
      method: 'DELETE',
      headers: getAuthHeaders(),
    });
    if (!res.ok) {
      const data = await res.json();
      throw new Error(data.error);
    }
    await refreshServers();
    if (selectedServerId === id) {
      setSelectedServerId('local');
    }
  }, [getAuthHeaders, refreshServers, selectedServerId]);

  const testConnection = useCallback(async (id: string) => {
    const res = await fetch(`/api/remote-servers/${id}/test`, {
      method: 'POST',
      headers: getAuthHeaders(),
    });
    return res.json();
  }, [getAuthHeaders]);

  const deployBroker = useCallback(async (id: string) => {
    const res = await fetch(`/api/remote-servers/${id}/deploy`, {
      method: 'POST',
      headers: getAuthHeaders(),
    });
    return res.json();
  }, [getAuthHeaders]);

  // Initial load
  useEffect(() => {
    refreshServers();
  }, [refreshServers]);

  // Poll server statuses
  useEffect(() => {
    const interval = setInterval(async () => {
      const statuses: Record<string, TunnelStatus> = {};
      for (const server of servers) {
        try {
          const res = await fetch(`/api/remote-servers/${server.id}/status`, {
            headers: getAuthHeaders(),
          });
          if (res.ok) {
            const data = await res.json();
            statuses[server.id] = data.status as TunnelStatus;
          }
        } catch {
          statuses[server.id] = 'disconnected';
        }
      }
      setServerStatuses(statuses);
    }, 10000); // Poll every 10s

    return () => clearInterval(interval);
  }, [servers, getAuthHeaders]);

  const selectedServer = servers.find(s => s.id === selectedServerId) || null;

  return (
    <ServerContext.Provider
      value={{
        servers,
        selectedServerId,
        selectedServer,
        serverStatuses,
        setSelectedServerId,
        refreshServers,
        addServer,
        updateServer,
        deleteServer,
        testConnection,
        deployBroker,
      }}
    >
      {children}
    </ServerContext.Provider>
  );
}

export function useServer() {
  const ctx = useContext(ServerContext);
  if (!ctx) throw new Error('useServer must be used within ServerProvider');
  return ctx;
}
