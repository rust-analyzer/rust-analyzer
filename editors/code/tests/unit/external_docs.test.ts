import * as assert from "node:assert/strict";

import { normalizeLocalDocsUri } from "../../src/external_docs";
import type { Context } from ".";

export async function getTests(ctx: Context) {
    await ctx.suite("External documentation URIs", (suite) => {
        suite.addTest("rewrites WSL files to host-readable UNC URIs", async () => {
            assert.equal(
                normalizeLocalDocsUri(
                    "file:///home/user/target/doc/foo.html",
                    "wsl+Ubuntu-22.04",
                    "vscode-remote",
                ),
                "file://///wsl.localhost/Ubuntu-22.04/home/user/target/doc/foo.html",
            );
        });

        suite.addTest("preserves encoded paths and URI components", async () => {
            assert.equal(
                normalizeLocalDocsUri(
                    "file:///home/user/my%20crate/doc.html?section=1#intro",
                    "ssh-remote+host",
                    "vscode-remote",
                ),
                "vscode-remote://ssh-remote+host/home/user/my%20crate/doc.html:1:1?section=1#intro",
            );
        });

        suite.addTest("leaves local and non-file URIs unchanged", async () => {
            assert.equal(
                normalizeLocalDocsUri("file:///tmp/doc.html", undefined, "vscode-remote"),
                "file:///tmp/doc.html",
            );
            assert.equal(
                normalizeLocalDocsUri("https://docs.rs/foo", "wsl+Ubuntu", "vscode-remote"),
                "https://docs.rs/foo",
            );
            assert.equal(
                normalizeLocalDocsUri(
                    "vscode-remote://wsl+Ubuntu/home/user/doc.html",
                    "wsl+Ubuntu",
                    "vscode-remote",
                ),
                "vscode-remote://wsl+Ubuntu/home/user/doc.html",
            );
            assert.equal(
                normalizeLocalDocsUri("file:///tmp/doc.html", "wsl+Ubuntu", "file"),
                "file:///tmp/doc.html",
            );
        });

        suite.addTest("preserves file URI authorities and encodes remote authorities", async () => {
            assert.equal(
                normalizeLocalDocsUri(
                    "file://server/share/doc.html",
                    "ssh-remote+user@host#1",
                    "vscode-remote",
                ),
                "vscode-remote://ssh-remote+user%40host%231//server/share/doc.html:1:1",
            );
        });
    });
}
