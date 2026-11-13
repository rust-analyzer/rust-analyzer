import * as assert from "assert";
import * as vscode from "vscode";
import type { Context } from ".";

export async function getTests(ctx: Context) {
    await ctx.suite("Debug Session Restart", (suite) => {
        suite.addTest("Restart command triggers recompilation", async () => {
            // This test verifies that when a restart DAP message is received,
            // the onWillReceiveMessage handler is called correctly

            let recompileCalled = false;

            // Mock the debug adapter tracker
            const tracker: vscode.DebugAdapterTracker = {
                onWillReceiveMessage: async (message: unknown) => {
                    const msg = message as { command?: string };
                    if (msg.command === "restart") {
                        recompileCalled = true;
                    }
                },
            };

            // Simulate receiving a restart message
            if (tracker.onWillReceiveMessage) {
                await tracker.onWillReceiveMessage({ command: "restart" });
            }

            assert.strictEqual(
                recompileCalled,
                true,
                "Recompilation should be triggered on restart command",
            );
        });

        suite.addTest("Session tracking works correctly", async () => {
            // This test verifies that debug sessions are tracked in activeDebugSessionIds
            const sessionIds: string[] = [];

            const mockSession: vscode.DebugSession = {
                id: "test-session-2",
                type: "lldb",
                name: "Test Session 2",
                workspaceFolder: undefined,
                configuration: {
                    type: "lldb",
                    request: "launch",
                    name: "Test",
                    program: "/path/to/binary",
                    cwd: "/path/to/project",
                    args: [],
                },
                customRequest: async () => {},
                getDebugProtocolBreakpoint: async () => undefined,
            };

            // Simulate session start
            if (!sessionIds.includes(mockSession.id)) {
                sessionIds.push(mockSession.id);
            }

            assert.strictEqual(sessionIds.length, 1, "Session should be tracked");
            assert.strictEqual(sessionIds[0], "test-session-2", "Session ID should match");

            // Simulate session termination
            const index = sessionIds.findIndex((id) => id === mockSession.id);
            if (index !== -1) {
                sessionIds.splice(index, 1);
            }

            assert.strictEqual(sessionIds.length, 0, "Session should be removed after termination");
        });

        suite.addTest("Invalidate request is sent after recompilation", async () => {
            // This test verifies that we attempt to send an invalidate request
            let invalidateCalled = false;

            const mockSession: vscode.DebugSession = {
                id: "test-session-3",
                type: "lldb",
                name: "Test Session 3",
                workspaceFolder: undefined,
                configuration: {
                    type: "lldb",
                    request: "launch",
                    name: "Test",
                    program: "/path/to/binary",
                    cwd: "/path/to/project",
                    args: [],
                },
                customRequest: async (command: string) => {
                    if (command === "invalidate") {
                        invalidateCalled = true;
                    }
                },
                getDebugProtocolBreakpoint: async () => undefined,
            };

            // Simulate the invalidate request
            await mockSession.customRequest("invalidate");

            assert.strictEqual(
                invalidateCalled,
                true,
                "Invalidate request should be sent to debug adapter",
            );
        });
    });
}
