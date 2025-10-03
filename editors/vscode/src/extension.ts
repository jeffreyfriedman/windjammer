import * as path from 'path';
import { workspace, ExtensionContext, window, commands } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    console.log('Windjammer extension is now active');

    // Get the server path from configuration
    const config = workspace.getConfiguration('windjammer');
    const serverPath = config.get<string>('server.path') || 'windjammer-lsp';

    // Server options
    const serverOptions: ServerOptions = {
        command: serverPath,
        args: [],
        transport: TransportKind.stdio
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'windjammer' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.wj')
        }
    };

    // Create the language client
    client = new LanguageClient(
        'windjammer',
        'Windjammer Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client
    client.start();

    // Register commands
    context.subscriptions.push(
        commands.registerCommand('windjammer.restartServer', async () => {
            await client.stop();
            await client.start();
            window.showInformationMessage('Windjammer Language Server restarted');
        })
    );

    context.subscriptions.push(
        commands.registerCommand('windjammer.showGeneratedRust', async () => {
            const editor = window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'windjammer') {
                window.showErrorMessage('No Windjammer file is currently open');
                return;
            }

            // TODO: Implement showing generated Rust code
            window.showInformationMessage(
                'Generated Rust code view coming soon!'
            );
        })
    );

    // Status bar item
    const statusBar = window.createStatusBarItem();
    statusBar.text = '$(rocket) Windjammer';
    statusBar.tooltip = 'Windjammer Language Server';
    statusBar.show();
    context.subscriptions.push(statusBar);

    // Update status when client is ready
    client.onReady().then(() => {
        statusBar.text = '$(check) Windjammer';
        statusBar.tooltip = 'Windjammer Language Server is running';
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

