import { URL } from "node:url";

/**
 * Make a server-local documentation URI openable by a remote VS Code client.
 *
 * The language server can only describe a local file with a `file://` URI. When
 * the extension host is remote (for example, WSL), that URI points to the
 * remote filesystem and cannot be opened by the local system browser. VS Code
 * exposes its remote filesystem through a host-readable UNC file URI for WSL,
 * while other remote authorities use a `vscode-remote://` URI with a file
 * position. The latter is important because VS Code treats a remote URI
 * without a `:line:column` suffix as a folder when opened through the protocol.
 */
export function normalizeLocalDocsUri(
    docLink: string,
    remoteAuthority: string | undefined,
    documentScheme: string | undefined,
): string {
    if (
        !remoteAuthority ||
        documentScheme !== "vscode-remote" ||
        !docLink.toLowerCase().startsWith("file://")
    ) {
        return docLink;
    }

    let uri: URL;
    try {
        uri = new URL(docLink);
    } catch {
        return docLink;
    }

    if (remoteAuthority.toLowerCase().startsWith("wsl+")) {
        const distribution = encodeURIComponent(remoteAuthority.slice(4));
        return `file://///wsl.localhost/${distribution}${uri.pathname}${uri.search}${uri.hash}`;
    }

    const authority = encodeURIComponent(remoteAuthority).replaceAll("%2B", "+");
    const path = uri.hostname ? `//${uri.hostname}${uri.pathname}` : uri.pathname;
    return `vscode-remote://${authority}${path}:1:1${uri.search}${uri.hash}`;
}
