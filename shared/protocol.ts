// Shared protocol types between frontend and backend
// These types mirror the Rust serde types in backend/src/ws/protocol.rs

// --- Inbound messages (Frontend → Backend) ---

export type InboundMessage =
  | ClaudeCommandMessage
  | CursorCommandMessage
  | CodexCommandMessage
  | GeminiCommandMessage
  | AbortSessionMessage
  | CheckSessionStatusMessage
  | PermissionResponseMessage;

export interface ClaudeCommandMessage {
  type: 'claude-command';
  command: string;
  projectPath?: string;
  cwd?: string;
  sessionId?: string;
  resume?: boolean;
  model?: string;
  serverId?: string;
  maxTurns?: number;
  allowedTools?: string[];
  systemPrompt?: string;
  appendSystemPrompt?: string;
  permissionMode?: string;
  continueConversation?: boolean;
}

export interface CursorCommandMessage {
  type: 'cursor-command';
  command: string;
  projectPath?: string;
  cwd?: string;
  sessionId?: string;
  model?: string;
  serverId?: string;
}

export interface CodexCommandMessage {
  type: 'codex-command';
  command: string;
  projectPath?: string;
  cwd?: string;
  sessionId?: string;
  model?: string;
  serverId?: string;
}

export interface GeminiCommandMessage {
  type: 'gemini-command';
  command: string;
  projectPath?: string;
  cwd?: string;
  sessionId?: string;
  model?: string;
  serverId?: string;
}

export interface AbortSessionMessage {
  type: 'abort-session';
  sessionId: string;
  provider: string;
}

export interface CheckSessionStatusMessage {
  type: 'check-session-status';
  sessionId: string;
  provider: string;
}

export interface PermissionResponseMessage {
  type: 'permission-response';
  requestId: string;
  approved: boolean;
}

// --- Outbound messages (Backend → Frontend) ---

export type OutboundMessage =
  | SessionCreatedMessage
  | ClaudeResponseMessage
  | TokenBudgetMessage
  | ClaudeCompleteMessage
  | ClaudeErrorMessage
  | PermissionRequestMessage
  | CursorResponseMessage
  | CursorCompleteMessage
  | CodexResponseMessage
  | CodexCompleteMessage
  | GeminiResponseMessage
  | GeminiCompleteMessage
  | ErrorMessage;

export interface SessionCreatedMessage {
  type: 'session-created';
  sessionId: string;
}

export interface ClaudeResponseMessage {
  type: 'claude-response';
  data: any;
  sessionId: string;
}

export interface TokenBudgetMessage {
  type: 'token-budget';
  data: any;
  sessionId: string;
}

export interface ClaudeCompleteMessage {
  type: 'claude-complete';
  sessionId: string;
  exitCode: number;
}

export interface ClaudeErrorMessage {
  type: 'claude-error';
  error: string;
  sessionId?: string;
}

export interface PermissionRequestMessage {
  type: 'permission-request';
  requestId: string;
  toolName: string;
  params: any;
}

export interface CursorResponseMessage {
  type: 'cursor-response';
  data: any;
  sessionId: string;
}

export interface CursorCompleteMessage {
  type: 'cursor-complete';
  sessionId: string;
  exitCode: number;
}

export interface CodexResponseMessage {
  type: 'codex-response';
  data: any;
  sessionId: string;
}

export interface CodexCompleteMessage {
  type: 'codex-complete';
  sessionId: string;
  exitCode: number;
}

export interface GeminiResponseMessage {
  type: 'gemini-response';
  data: any;
  sessionId: string;
}

export interface GeminiCompleteMessage {
  type: 'gemini-complete';
  sessionId: string;
  exitCode: number;
}

export interface ErrorMessage {
  type: 'error';
  error: string;
}

// --- Server types ---

export interface Server {
  id: string;
  name: string;
  isLocal: boolean;
  hostname: string;
  sshPort: number;
  sshUser: string;
  sshKeyPath?: string;
  authMethod: string;
  brokerPort: number;
  defaultWorkDir?: string;
  tunnelLocalPort?: number;
  autoUpdate: boolean;
  idleTimeoutSecs: number;
  isActive: boolean;
  brokerVersion?: string;
  lastConnectedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export type TunnelStatus = 'connected' | 'connecting' | 'disconnected' | 'error';
