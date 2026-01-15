/**
 * SSH connection related type definitions
 */

export interface SSHConnectionData {
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
}

export interface SSHConnection {
  id: string;
  name: string;
  data: SSHConnectionData;
  lastConnected?: Date;
}
