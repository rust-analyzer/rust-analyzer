import * as assert from "assert";
import { _private } from "../../src/outdated_toolchain";
import type { Context } from ".";

/** Mirrors the message built by `GlobalState::check_workspaces_msrv`. */
function serverMessage(manifestPath: string): string {
    return (
        `Workspace \`${manifestPath}\` is using an outdated toolchain version \`1.80.0\` but rust-analyzer only supports \`1.94.0\` and higher.\n` +
        "Consider using the rust-analyzer rustup component for your toolchain or\n" +
        "                        upgrade your toolchain to a supported version.\n\n"
    );
}

export async function getTests(ctx: Context) {
    await ctx.suite("Outdated toolchain/Parse server message", (suite) => {
        suite.addTest("Cargo workspace", async () => {
            assert.deepStrictEqual(
                _private.parseServerMessage(serverMessage("/home/user/repo/Cargo.toml")),
                "/home/user/repo/Cargo.toml",
            );
        });

        suite.addTest("Path containing a backtick", async () => {
            assert.deepStrictEqual(
                _private.parseServerMessage(serverMessage("/home/user/re`po/Cargo.toml")),
                "/home/user/re`po/Cargo.toml",
            );
        });

        suite.addTest("Other server message", async () => {
            assert.deepStrictEqual(
                _private.parseServerMessage("Failed to discover workspace."),
                undefined,
            );
        });
    });
}
