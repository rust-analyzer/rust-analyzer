import * as vscode from 'vscode';
import * as path from 'path';
import { func } from 'vscode-languageclient/lib/utils/is';


const iconsRootPath = path.join(path.dirname(__dirname), '..', 'resources', 'icons'); 

function getIconUri(iconName: string, theme: string): vscode.Uri {
	return vscode.Uri.file(path.join(iconsRootPath, theme, `${iconName}.svg`));
}

interface Workspace {
    crates: Crate[],
    location: string,
}

interface Crate {
    name: string,
    modules: Module[], 
    location: string,
}

interface Module {
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
    name: string,
    location: string,
    kind: TestKind,
}

class Workspace extends vscode.TreeItem {
    get tooltip(): string {
        // @ts-ignore
        return super.label;
    }
    
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
        name: string,
        modules: Module[], 
    ) {
        super(name, vscode.TreeItemCollapsibleState.Collapsed);
    }

    get tooltip(): string {
        // @ts-ignore
        return super.label;
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
        name: string,
        location: string,
        modules?: Module[], 
        targets?: Function[],
    ) {
        super(name, vscode.TreeItemCollapsibleState.Collapsed);
        this.location = location;
    }   

    get tooltip(): string {
        // @ts-ignore
        return super.label;
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
        name: string,
        location: string,
        kind: TestKind,
    ) {
        super(name, vscode.TreeItemCollapsibleState.None);
        this.location = location;
        
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
    
    get tooltip(): string {
        // @ts-ignore
        return super.label;
    }
    
    get description(): string {
        return this.location;
    }

    getChildren(): null {
        return null;
    }   
}

type Executable = Workspace | Crate | Module | Function;

export class RunnableProvider implements vscode.TreeDataProvider<Executable> {
    // private changesEmitter: vscode.EventEmitter<Executable | undefined> = new vscode.EventEmitter<Executable | undefined>();
    // readonly onDidChangeTreeData: vscode.Event<Executable | undefined> = this.changesEmitter.event;

    getChildren(element?: Executable): vscode.ProviderResult<Executable[]> {
        if(element == undefined) {
            return Promise.resolve([]); 
        }
        
        return element.getChildren();
    }
    
    getTreeItem(element: Executable): Executable {
        return element;
    }   
}

function runExecutable() {
    
}

function goToDefinition() {
    vscode.workspace.openTextDocument().then((document)=>{
        vscode.window.activeTextEditor?.revealRange();    
    });
} 




