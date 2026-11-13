import * as os from "os";
import * as vscode from "vscode";
import * as path from "path";
import type * as ra from "./lsp_ext";

import { Cargo } from "./toolchain";
import type { Ctx } from "./ctx";
import { createTaskFromRunnable, prepareEnv } from "./run";
import {
    execute,
    isCargoRunnableArgs,
    unwrapUndefinable,
    log,
    normalizeDriveLetter,
    Env,
} from "./util";
import type { Config } from "./config";

export async function makeDebugConfig(ctx: Ctx, runnable: ra.Runnable): Promise<void> {
    const scope = ctx.activeRustEditor?.document.uri;
    if (!scope) return;

    const debugConfig = await getDebugConfiguration(ctx.config, runnable, false);
    if (!debugConfig) return;

    const wsLaunchSection = vscode.workspace.getConfiguration("launch", scope);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const configurations = wsLaunchSection.get<any[]>("configurations") || [];

    const index = configurations.findIndex((c) => c.name === debugConfig.name);
    if (index !== -1) {
        const answer = await vscode.window.showErrorMessage(
            `Launch configuration '${debugConfig.name}' already exists!`,
            "Cancel",
            "Update",
        );
        if (answer === "Cancel") return;

        configurations[index] = debugConfig;
    } else {
        configurations.push(debugConfig);
    }

    await wsLaunchSection.update("configurations", configurations);
}

export async function startDebugSession(ctx: Ctx, runnable: ra.Runnable): Promise<boolean> {
    let debugConfig: vscode.DebugConfiguration | undefined;
    let message = "";

    const wsLaunchSection = vscode.workspace.getConfiguration("launch");
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const configurations = wsLaunchSection.get<any[]>("configurations") || [];

    // The runnable label is the name of the test with the "test prefix"
    // e.g. test test_feature_x
    const index = configurations.findIndex((c) => c.name === runnable.label);
    if (-1 !== index) {
        debugConfig = configurations[index];
        message = " (from launch.json)";
    } else {
        debugConfig = await getDebugConfiguration(ctx.config, runnable);
    }

    if (!debugConfig) return false;

    log.debug(`Launching debug configuration${message}:`);
    log.debug(JSON.stringify(debugConfig, null, 2));
    return vscode.debug.startDebugging(undefined, debugConfig);
}

function createCommandLink(extensionId: string): string {
    // do not remove the second quotes inside
    // encodeURIComponent or it won't work
    return `extension.open?${encodeURIComponent(`"${extensionId}"`)}`;
}

async function getDebugConfiguration(
    config: Config,
    runnable: ra.Runnable,
    inheritEnv: boolean = true,
): Promise<vscode.DebugConfiguration | undefined> {
    if (!isCargoRunnableArgs(runnable.args)) {
        return;
    }
    const runnableArgs: ra.CargoRunnableArgs = runnable.args;

    const debugOptions = config.debug;

    let provider: null | KnownEnginesType = null;

    if (debugOptions.engine === "auto") {
        for (const engineId in knownEngines) {
            const debugEngine = vscode.extensions.getExtension(engineId);
            if (debugEngine) {
                provider = knownEngines[engineId as keyof typeof knownEngines];
                break;
            }
        }
    } else if (debugOptions.engine) {
        const debugEngine = vscode.extensions.getExtension(debugOptions.engine);
        if (debugEngine && Object.keys(knownEngines).includes(debugOptions.engine)) {
            provider = knownEngines[debugOptions.engine as keyof typeof knownEngines];
        }
    }

    if (!provider) {
        const commandCCpp: string = createCommandLink("ms-vscode.cpptools");
        const commandCodeLLDB: string = createCommandLink("vadimcn.vscode-lldb");
        const commandNativeDebug: string = createCommandLink("webfreak.debug");
        const commandLLDBDap: string = createCommandLink("llvm-vs-code-extensions.lldb-dap");

        await vscode.window.showErrorMessage(
            `Install [CodeLLDB](command:${commandCodeLLDB} "Open CodeLLDB")` +
                `, [lldb-dap](command:${commandLLDBDap} "Open lldb-dap")` +
                `, [C/C++](command:${commandCCpp} "Open C/C++") ` +
                `or [Native Debug](command:${commandNativeDebug} "Open Native Debug") for debugging.`,
        );
        return;
    }

    // folder exists or RA is not active.

    const workspaceFolders = vscode.workspace.workspaceFolders!;
    const isMultiFolderWorkspace = workspaceFolders.length > 1;
    const firstWorkspace = workspaceFolders[0];
    const maybeWorkspace =
        !isMultiFolderWorkspace || !runnableArgs.workspaceRoot
            ? firstWorkspace
            : workspaceFolders.find((w) => runnableArgs.workspaceRoot?.includes(w.uri.fsPath)) ||
              firstWorkspace;

    const workspace = unwrapUndefinable(maybeWorkspace);
    const wsFolder = normalizeDriveLetter(path.normalize(workspace.uri.fsPath));

    const workspaceQualifier = isMultiFolderWorkspace ? `:${workspace.name}` : "";
    function simplifyPath(p: string): string {
        // in windows, the drive letter can vary in casing for VSCode, so we gotta normalize that first
        p = normalizeDriveLetter(path.normalize(p));
        // see https://github.com/rust-lang/rust-analyzer/pull/5513#issuecomment-663458818 for why this is needed
        return p.replace(wsFolder, `\${workspaceFolder${workspaceQualifier}}`);
    }

    const executable = await getDebugExecutable(
        runnableArgs,
        prepareEnv(true, {}, config.runnablesExtraEnv(runnable.label)),
    );

    const env = prepareEnv(
        inheritEnv,
        runnableArgs.environment,
        config.runnablesExtraEnv(runnable.label),
    );
    let sourceFileMap = debugOptions.sourceFileMap;

    if (sourceFileMap === "auto") {
        sourceFileMap = {};
        const computedSourceFileMap = await discoverSourceFileMap(env, wsFolder);

        if (computedSourceFileMap) {
            // lldb-dap requires passing the source map as an array of two element arrays.
            // the two element array contains a source and destination pathname.
            // TODO: remove lldb-dap-specific post-processing once
            // https://github.com/llvm/llvm-project/pull/106919/ is released in the extension.
            if (provider.type === "lldb-dap") {
                provider.additional["sourceMap"] = [
                    [computedSourceFileMap?.source, computedSourceFileMap?.destination],
                ];
            } else {
                sourceFileMap[computedSourceFileMap.source] = computedSourceFileMap.destination;
            }
        }
    }

    const debugConfig = getDebugConfig(
        provider,
        simplifyPath,
        runnable,
        runnableArgs,
        executable,
        env,
        sourceFileMap,
    );
    if (debugConfig.type in debugOptions.engineSettings) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const settingsMap = (debugOptions.engineSettings as any)[debugConfig.type];
        for (const key in settingsMap) {
            debugConfig[key] = settingsMap[key];
        }
    }

    if (debugConfig.name === "run binary") {
        // The LSP side: crates\rust-analyzer\src\main_loop\handlers.rs,
        // fn to_lsp_runnable(...) with RunnableKind::Bin
        // FIXME: Neither crates\rust-analyzer\src\main_loop\handlers.rs
        // nor to_lsp_runnable exist anymore
        debugConfig.name = `run ${path.basename(executable)}`;
    }

    const cwd = debugConfig["cwd"];
    if (cwd) {
        debugConfig["cwd"] = simplifyPath(cwd);
    }

    return debugConfig;
}

type SourceFileMap = {
    source: string;
    destination: string;
};

async function discoverSourceFileMap(env: Env, cwd: string): Promise<SourceFileMap | undefined> {
    const sysroot = env["RUSTC_TOOLCHAIN"];
    if (sysroot) {
        // let's try to use the default toolchain
        const data = await execute(`rustc -V -v`, { cwd, env });
        const rx = /commit-hash:\s(.*)$/m;

        const commitHash = rx.exec(data)?.[1];
        if (commitHash) {
            const rustlib = path.normalize(sysroot + "/lib/rustlib/src/rust");
            return { source: "/rustc/" + commitHash, destination: rustlib };
        }
    }

    return;
}

type PropertyFetcher<Config, Input, Key extends keyof Config> = (
    input: Input,
) => [Key, Config[Key]];

type DebugConfigProvider<Type extends string, DebugConfig extends BaseDebugConfig<Type>> = {
    executableProperty: keyof DebugConfig;
    environmentProperty: PropertyFetcher<DebugConfig, Env, keyof DebugConfig>;
    runnableArgsProperty: PropertyFetcher<DebugConfig, ra.CargoRunnableArgs, keyof DebugConfig>;
    sourceFileMapProperty?: keyof DebugConfig;
    type: Type;
    additional: Record<string, unknown>;
};

type KnownEnginesType = (typeof knownEngines)[keyof typeof knownEngines];
const knownEngines: {
    "llvm-vs-code-extensions.lldb-dap": DebugConfigProvider<"lldb-dap", LldbDapDebugConfig>;
    "vadimcn.vscode-lldb": DebugConfigProvider<"lldb", CodeLldbDebugConfig>;
    "ms-vscode.cpptools": DebugConfigProvider<"cppvsdbg" | "cppdbg", CCppDebugConfig>;
    "webfreak.debug": DebugConfigProvider<"gdb", NativeDebugConfig>;
} = {
    "llvm-vs-code-extensions.lldb-dap": {
        type: "lldb-dap",
        executableProperty: "program",
        environmentProperty: (env) => ["env", Object.entries(env).map(([k, v]) => `${k}=${v}`)],
        runnableArgsProperty: (runnableArgs: ra.CargoRunnableArgs) => [
            "args",
            runnableArgs.executableArgs,
        ],
        additional: {},
    },
    "vadimcn.vscode-lldb": {
        type: "lldb",
        executableProperty: "program",
        environmentProperty: (env) => ["env", env],
        runnableArgsProperty: (runnableArgs: ra.CargoRunnableArgs) => [
            "args",
            runnableArgs.executableArgs,
        ],
        sourceFileMapProperty: "sourceMap",
        additional: {
            sourceLanguages: ["rust"],
        },
    },
    "ms-vscode.cpptools": {
        type: os.platform() === "win32" ? "cppvsdbg" : "cppdbg",
        executableProperty: "program",
        environmentProperty: (env) => [
            "environment",
            Object.entries(env).map((entry) => ({
                name: entry[0],
                value: entry[1] ?? "",
            })),
        ],
        runnableArgsProperty: (runnableArgs: ra.CargoRunnableArgs) => [
            "args",
            runnableArgs.executableArgs,
        ],
        sourceFileMapProperty: "sourceFileMap",
        additional: {
            osx: {
                MIMode: "lldb",
            },
        },
    },
    "webfreak.debug": {
        type: "gdb",
        executableProperty: "target",
        runnableArgsProperty: (runnableArgs: ra.CargoRunnableArgs) => [
            "arguments",
            quote(runnableArgs.executableArgs),
        ],
        environmentProperty: (env) => ["env", env],
        additional: {
            valuesFormatting: "prettyPrinters",
        },
    },
};

async function getDebugExecutable(runnableArgs: ra.CargoRunnableArgs, env: Env): Promise<string> {
    const cargo = new Cargo(runnableArgs.workspaceRoot || ".", env);
    const executable = await cargo.executableFromArgs(runnableArgs);

    // if we are here, there were no compilation errors.
    return executable;
}

type BaseDebugConfig<type extends string> = {
    type: type;
    request: "launch";
    name: string;
    cwd: string;
};

function getDebugConfig(
    provider: KnownEnginesType,
    simplifyPath: (p: string) => string,
    runnable: ra.Runnable,
    runnableArgs: ra.CargoRunnableArgs,
    executable: string,
    env: Env,
    sourceFileMap?: Record<string, string>,
): vscode.DebugConfiguration {
    const {
        environmentProperty,
        executableProperty,
        runnableArgsProperty,
        type,
        additional,
        sourceFileMapProperty,
    } = provider;
    const [envProperty, envValue] = environmentProperty(env);
    const [argsProperty, argsValue] = runnableArgsProperty(runnableArgs);
    return {
        type,
        request: "launch",
        name: runnable.label,
        cwd: simplifyPath(runnable.args.cwd || runnableArgs.workspaceRoot || "."),
        [executableProperty]: simplifyPath(executable),
        [envProperty]: envValue,
        [argsProperty]: argsValue,
        ...(sourceFileMapProperty ? { [sourceFileMapProperty]: sourceFileMap } : {}),
        ...additional,
    };
}

type CCppDebugConfig = {
    program: string;
    args: string[];
    sourceFileMap: Record<string, string> | undefined;
    environment: {
        name: string;
        value: string;
    }[];
    // See https://github.com/rust-lang/rust-analyzer/issues/16901#issuecomment-2024486941
    osx: {
        MIMode: "lldb";
    };
} & BaseDebugConfig<"cppvsdbg" | "cppdbg">;

type LldbDapDebugConfig = {
    program: string;
    args: string[];
    env: string[];
    sourceMap: [string, string][];
} & BaseDebugConfig<"lldb-dap">;

type CodeLldbDebugConfig = {
    program: string;
    args: string[];
    sourceMap: Record<string, string> | undefined;
    sourceLanguages: ["rust"];
    env: Env;
} & BaseDebugConfig<"lldb">;

type NativeDebugConfig = {
    target: string;
    // See https://github.com/WebFreak001/code-debug/issues/359
    arguments: string;
    env: Env;
    valuesFormatting: "prettyPrinters";
} & BaseDebugConfig<"gdb">;

// Based on https://github.com/ljharb/shell-quote/blob/main/quote.js
function quote(xs: string[]) {
    return xs
        .map(function (s) {
            if (/["\s]/.test(s) && !/'/.test(s)) {
                return "'" + s.replace(/(['\\])/g, "\\$1") + "'";
            }
            if (/["'\s]/.test(s)) {
                return `"${s.replace(/(["\\$`!])/g, "\\$1")}"`;
            }
            return s.replace(/([A-Za-z]:)?([#!"$&'()*,:;<=>?@[\\\]^`{|}])/g, "$1\\$2");
        })
        .join(" ");
}

async function recompileTestFromDebuggingSession(session: vscode.DebugSession, ctx: Ctx) {
    const config: vscode.DebugConfiguration = session.configuration;
    const { cwd } = config;

    // Rebuild the entire project to ensure all changes are included
    const args: ra.CargoRunnableArgs = {
        cwd: cwd,
        cargoArgs: ["build", "--all-targets"],
        executableArgs: [],
    };
    const runnable: ra.Runnable = {
        kind: "cargo",
        label: "recompile-for-debug",
        args,
    };
    const task: vscode.Task = await createTaskFromRunnable(runnable, ctx.config);

    // Execute the build task and wait for it to complete
    const execution = await vscode.tasks.executeTask(task);

    return new Promise<void>((resolve, reject) => {
        const disposable = vscode.tasks.onDidEndTask((e) => {
            if (e.execution === execution) {
                disposable.dispose();
                resolve();
            }
        });

        // Add a timeout to prevent hanging forever
        setTimeout(() => {
            disposable.dispose();
            reject(new Error("Compilation timed out after 2 minutes"));
        }, 120000);
    });
}

export function initializeDebugSessionTrackingAndRebuild(ctx: Ctx) {
    // Track sessions we're manually restarting to avoid loops
    const manuallyRestartingSessions = new Set<string>();

    // Register a debug adapter tracker factory to intercept restart messages.
    // When the user clicks the restart button in the debug toolbar, VS Code sends a "restart"
    // command via the Debug Adapter Protocol (DAP). We intercept this to recompile the code
    // before restarting, ensuring the debugger uses the latest binary.
    //
    // Note: We must stop the session and start fresh (rather than just waiting for compilation)
    // because debug adapters cache the binary and symbols. A simple restart would use stale data.
    vscode.debug.registerDebugAdapterTrackerFactory("*", {
        createDebugAdapterTracker(session: vscode.DebugSession) {
            return {
                onWillReceiveMessage: async (message: unknown) => {
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    const msg = message as any;

                    // Intercept restart command - stop it and do our own restart
                    if (msg.command === "restart" && !manuallyRestartingSessions.has(session.id)) {
                        manuallyRestartingSessions.add(session.id);

                        // Stop the session immediately to clear debugger cache
                        vscode.debug.stopDebugging(session).then(async () => {
                            try {
                                // Show progress notification
                                await vscode.window.withProgress(
                                    {
                                        location: vscode.ProgressLocation.Notification,
                                        title: "Recompiling before debug restart",
                                        cancellable: false,
                                    },
                                    async (progress) => {
                                        progress.report({ increment: 0 });
                                        await recompileTestFromDebuggingSession(session, ctx);
                                        progress.report({ increment: 100 });
                                    },
                                );

                                // Start a completely fresh debug session with the same config
                                const started = await vscode.debug.startDebugging(
                                    session.workspaceFolder,
                                    session.configuration,
                                );

                                if (!started) {
                                    void vscode.window.showErrorMessage(
                                        "Failed to restart debug session",
                                    );
                                }
                            } catch (error) {
                                const errorMsg =
                                    error instanceof Error ? error.message : String(error);
                                void vscode.window.showErrorMessage(
                                    `Failed to recompile before restart: ${errorMsg}`,
                                );
                                log.error("Recompile and restart failed:", error);
                            } finally {
                                manuallyRestartingSessions.delete(session.id);
                            }
                        });

                        // Return false or modify message to cancel the original restart
                        // (though stopping the session should handle this)
                    }
                },
            };
        },
    });
}
