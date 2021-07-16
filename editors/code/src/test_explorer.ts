import * as vscode from 'vscode';
import * as path from 'path';
import { func } from 'vscode-languageclient/lib/utils/is';
import { strict } from 'assert';
import { Func } from 'mocha';


const iconsRootPath = path.join(path.dirname(__dirname), '..', 'resources', 'icons'); 

function getIconUri(iconName: string, theme: string): vscode.Uri {
	return vscode.Uri.file(path.join(iconsRootPath, theme, `${iconName}.svg`));
}

interface Workspace {
    id: string, 
    crates: Crate[],
    location: string,
}

interface Crate {
    id: string, 
    name: string,
    modules: Module[], 
    location: string,
}

interface Module {
    id: string, 
    name: string,
    modules?: Module[], 
    targets?: Function[],
    location: string,
}

enum TestKind {
    Unit,
    Bench,
}

interface Function {
    id: string, 
    name: string,
    location: string,
    kind: TestKind,
}

class Workspace extends vscode.TreeItem {  
    get description(): string {
        return this.location;
    }

    iconPath = {
        light: getIconUri('squares', 'dark'),
        dark: getIconUri('squares', 'dark'),
    };

    getChildren(): Crate[]{
        return this.crates;
    }
}

class Crate extends vscode.TreeItem {
    constructor(
        id: string, 
        name: string,
        modules: Module[], 
        location: string,
    ) {
        super(name, vscode.TreeItemCollapsibleState.Collapsed);
        this.location = location;
        this.id = id;
    }
    
    get description(): string {
        return this.location;
    }

    iconPath = {
        light: getIconUri('squares', 'dark'),
        dark: getIconUri('squares', 'dark'),
    };

    getChildren(): Module[] {
        return this.modules;
    }
}

class Module extends vscode.TreeItem {
    constructor(
        id: string, 
        name: string,
        location: string,
        modules?: Module[], 
        targets?: Function[],
    ) {
        super(name, vscode.TreeItemCollapsibleState.Collapsed);
        this.location = location;
        this.id = id;
    }   

    get description(): string {
        return this.location;
    }

    iconPath = {
        light: getIconUri('squares', 'dark'),
        dark: getIconUri('squares', 'dark'),
    };

    getChildren(): (Function | Module)[] {
        var res: (Function | Module)[] = [];
        if(this.targets != undefined) {
            res.concat(this.targets);
        }
        if(this.modules != undefined) {
            res.concat(this.modules);
        }
        return res;
    }
}

class Function extends vscode.TreeItem {
    constructor(
        id: string, 
        name: string,
        location: string,
        kind: TestKind,
    ) {
        super(name, vscode.TreeItemCollapsibleState.None);
        this.location = location;
        this.id = id;
        
        switch(kind) {
            case TestKind.Bench: {
                this.iconPath = {
                    light: getIconUri('accelerator', 'dark'),
                    dark: getIconUri('accelerator', 'dark'),
                };
                break;
            }
            case TestKind.Unit: {
                this.iconPath = {
                    light: getIconUri('test_sheet', 'dark'),
                    dark: getIconUri('test_sheet', 'dark'),
                };
                break;
            }
        }
    }
       
    get description(): string {
        return this.location;
    }

    getChildren(): null {
        return null;
    }   
}

type Node = Workspace | Crate | Module | Function;

function bfs(root: Node, process: (node: Node) => void) {
    let queue: Array<Node> = [root];
    while(queue.length != 0) {
        let current = queue.pop();
        //@ts-ignore
        process(current);
        //@ts-ignore
        current.getChildren();
    }
}

export class RunnableDataProvider implements vscode.TreeDataProvider<Node> {
    private _onDidChangeTreeData: vscode.EventEmitter<Node | undefined> = new vscode.EventEmitter<Node | undefined>()
    readonly onDidChangeTreeData: vscode.Event<Node | undefined> = this._onDidChangeTreeData.event;

    getChildren(element?: Node): vscode.ProviderResult<Node[]> {
        if(element == undefined) {
            return Promise.resolve([]); 
        }
        
        return element.getChildren();
    }
    
    getTreeItem(element: Node): Node {
        return element;
    }   
}

export class RunnableView {
    private dataProvider: RunnableDataProvider;
    private data: Node;

    constructor(context: vscode.ExtensionContext) {
        this.dataProvider = new RunnableDataProvider()
    }

    public applyUpdate(deltaUpdate: Patch[]) {
        deltaUpdate.map((patch) => {
            switch(patch.kind) {
                case PatchKind.Create: 
                    find(patch.targetId);
                break;
                case PatchKind.Delete:
                    find();
                break;
                case PatchKind.Update:
                    find();
                break;
            }
        });    
    }
}

/// The view synchronized with RA data by delta updates. The update is an array   
/// of elementary actions called a `Patch`. After applying an update to the tree 
/// it will become synchronized.

type Patch = Delete | Update | Create;

enum PatchKind {
    Delete = "DELETE",
    Update = "UPDATE",
    Create = "CREATE"
}

interface Delete {
    kind: PatchKind.Delete,
    id: string,
} 

interface Update {
    kind: PatchKind.Update,
    payload: {
        name?: string,
        location?: string,
        kind?: TestKind,
    },
}

interface Create {
    kind: PatchKind.Create,
    targetId: string,
    payload: Node,
}

// function runExecutable() {
//     // TODO: implement 
// }

// function goToDefinition() {
//     vscode.workspace.openTextDocument().then((document)=>{
//         vscode.window.activeTextEditor?.revealRange();    
//     });
// } 
// Для Workspace и Crate show in explorer

// TODO: возможность создавать run lists
// TODO: Run History




