import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('Windjammer extension is now active!');

    // Get configuration
    const config = vscode.workspace.getConfiguration('windjammer');
    const lspPath = config.get<string>('lsp.path', 'windjammer-lsp');
    const traceLevel = config.get<string>('lsp.trace.server', 'off');

    // Server options
    const serverOptions: ServerOptions = {
        command: lspPath,
        args: [],
        transport: TransportKind.stdio
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'windjammer' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.wj')
        },
        initializationOptions: {
            diagnostics: config.get('diagnostics.enable', true),
            inlayHints: config.get('inlayHints.enable', true),
            autoFix: config.get('autoFix.enable', true)
        }
    };

    // Create the language client
    client = new LanguageClient(
        'windjammer',
        'Windjammer Language Server',
        serverOptions,
        clientOptions
    );

    // Set trace level
    if (traceLevel !== 'off') {
        client.setTrace(traceLevel === 'verbose' ? 2 : 1);
    }

    // Start the client
    client.start();

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('windjammer.restartServer', async () => {
            await client.stop();
            client.start();
            vscode.window.showInformationMessage('Windjammer Language Server restarted');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('windjammer.explainError', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                return;
            }

            // Get diagnostic at cursor position
            const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);
            const cursorPos = editor.selection.active;
            const diagnostic = diagnostics.find(d => d.range.contains(cursorPos));

            if (diagnostic && diagnostic.code) {
                const code = diagnostic.code.toString();
                const terminal = vscode.window.createTerminal('Windjammer Explain');
                terminal.show();
                terminal.sendText(`wj explain ${code}`);
            } else {
                vscode.window.showInformationMessage('No error code at cursor position');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('windjammer.showErrorCatalog', async () => {
            const terminal = vscode.window.createTerminal('Windjammer Docs');
            terminal.show();
            terminal.sendText('wj docs --format html && open docs/errors/index.html');
        })
    );

    // Status bar item
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.text = '$(check) Windjammer';
    statusBarItem.tooltip = 'Windjammer Language Server is running';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    console.log('Windjammer Language Server started successfully');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

