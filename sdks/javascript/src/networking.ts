/**
 * Networking components.
 */

/**
 * Network client.
 */
export class NetworkClient {
  /** Server address */
  serverAddress: string;
  
  /** Connection status */
  connected: boolean = false;

  /**
   * Create a new network client.
   * 
   * @param serverAddress - Server address to connect to
   */
  constructor(serverAddress: string = '127.0.0.1:7777') {
    this.serverAddress = serverAddress;
  }

  /**
   * Connect to the server.
   * 
   * @returns True if connection successful
   */
  connect(): boolean {
    console.log(`[Network] Connecting to ${this.serverAddress}...`);
    this.connected = true;
    return true;
  }

  /**
   * Disconnect from the server.
   */
  disconnect(): void {
    console.log('[Network] Disconnecting...');
    this.connected = false;
  }

  /**
   * Send a message to the server.
   * 
   * @param message - Message to send
   */
  send(message: Uint8Array): void {
    if (this.connected) {
      console.log(`[Network] Sending ${message.length} bytes`);
    }
  }

  toString(): string {
    return `NetworkClient(server='${this.serverAddress}', connected=${this.connected})`;
  }
}

/**
 * Network server.
 */
export class NetworkServer {
  /** Port to listen on */
  port: number;
  
  /** Server running status */
  running: boolean = false;

  /**
   * Create a new network server.
   * 
   * @param port - Port to listen on
   */
  constructor(port: number = 7777) {
    this.port = port;
  }

  /**
   * Start the server.
   * 
   * @returns True if server started successfully
   */
  start(): boolean {
    console.log(`[Network] Starting server on port ${this.port}...`);
    this.running = true;
    return true;
  }

  /**
   * Stop the server.
   */
  stop(): void {
    console.log('[Network] Stopping server...');
    this.running = false;
  }

  toString(): string {
    return `NetworkServer(port=${this.port}, running=${this.running})`;
  }
}

