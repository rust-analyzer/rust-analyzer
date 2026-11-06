import * as vscode from "vscode";
import { findRustToolchainFile } from "./util";

/**
 * Matches the message the server sends for each workspace whose toolchain is older than the
 * minimum it supports. Keep in sync with `GlobalState::check_workspaces_msrv`.
 */
const SERVER_MESSAGE_PATTERN =
    /^Workspace `(.+)` is using an outdated toolchain version `[^`]+` but rust-analyzer only supports/s;

/** Client-only setting that controls whether outdated toolchain notifications are shown. */
const SHOW_NOTIFICATION_SETTING = "showOutdatedToolchainNotification";

/**
 * Shown instead of the server's message for workspaces with a rust-toolchain file.
 *
 * The text does not mention the workspace, so VS Code shows a single notification no matter how
 * many workspaces have an outdated toolchain.
 */
const NOTIFICATION_MESSAGE =
    "Your workspace uses a rust-toolchain file with a toolchain too old for the extension shipped rust-analyzer to work properly. " +
    "Consider adding the rust-analyzer component to the toolchain file to use a compatible rust-analyzer version. " +
    "Add the following to your rust-toolchain file's `[toolchain]` section:\n" +
    'components = ["rust-analyzer"]';

/**
 * Returns the manifest path (or root, if there is no manifest) of the workspace that `message`
 * is about, or `undefined` if `message` is not an outdated toolchain message.
 */
function parseServerMessage(message: string): string | undefined {
    return SERVER_MESSAGE_PATTERN.exec(message)?.[1];
}

/**
 * Shows a notification for `message` if it is an outdated toolchain message from the server.
 *
 * Returns `false` if `message` is not an outdated toolchain message.
 */
export function handleOutdatedToolchainMessage(message: string): boolean {
    const manifestPath = parseServerMessage(message);
    if (manifestPath === undefined) {
        return false;
    }
    void showNotification(manifestPath, message);
    return true;
}

async function showNotification(manifestPath: string, serverMessage: string) {
    const config = vscode.workspace.getConfiguration("rust-analyzer");
    if (!config.get<boolean>(SHOW_NOTIFICATION_SETTING)) {
        return;
    }

    const toolchainFile = await findRustToolchainFile(manifestPath);
    // Without a rust-toolchain file the advice does not apply, so show the server's message at
    // the warning level it was sent with.
    if (!toolchainFile) {
        void vscode.window.showWarningMessage(serverMessage);
        return;
    }

    const openFile = "Open rust-toolchain file";
    const dontShowAgain = "Don't show this again";
    // Error level notifications with actions stay visible until the user dismisses them.
    const choice = await vscode.window.showErrorMessage(
        NOTIFICATION_MESSAGE,
        openFile,
        dontShowAgain,
    );
    if (choice === openFile) {
        await vscode.window.showTextDocument(toolchainFile);
    } else if (choice === dontShowAgain) {
        await config.update(SHOW_NOTIFICATION_SETTING, false, vscode.ConfigurationTarget.Global);
    }
}

export const _private = { parseServerMessage };
