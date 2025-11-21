import * as path from 'path';
import { workspace, ExtensionContext, window } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    // Get configuration
    const config = workspace.getConfiguration('windjammer');
    const serverPath = config.get<string>('lsp.serverPath', 'windjammer-lsp');
    const traceLevel = config.get<string>('lsp.trace.server', 'off');

    // Server options
    const serverOptions: ServerOptions = {
        command: serverPath,
        transport: TransportKind.stdio,
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'windjammer' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/.wj')
        },
        initializationOptions: {
            inlayHints: config.get<boolean>('inlayHints.enable', true),
            completion: config.get<boolean>('completion.enable', true)
        }
    };

    // Create the language client
    client = new LanguageClient(
        'windjammerLanguageServer',
        'Windjammer Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client (which will also start the server)
    client.start().then(() => {
        window.showInformationMessage('Windjammer LSP activated! 🌊');
        
        // Log ownership inference hints
        console.log('Windjammer: Ownership inference hints enabled');
        console.log('Windjammer: You will see inferred &, &mut, and owned annotations inline!');
    }).catch((error) => {
        window.showErrorMessage(`Failed to start Windjammer LSP: ${error.message}`);
        console.error('Windjammer LSP error:', error);
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
