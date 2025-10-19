use super::traits::EditorManager;
use super::vscode::VSCodeManager;
use super::jetbrains::JetBrainsManager;
use super::terminal::{VimManager, NeovimManager, EmacsManager};
use super::others::{XcodeManager, ZedManager, SublimeManager};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct EditorRegistry {
    managers: RwLock<HashMap<String, Arc<dyn EditorManager>>>,
}

impl EditorRegistry {
    pub fn new() -> Self {
        let registry = Self {
            managers: RwLock::new(HashMap::new()),
        };

        registry.register_all();
        registry
    }

    fn register_all(&self) {
        self.register(Arc::new(VSCodeManager::new("vscode", "Visual Studio Code", "code", "Visual Studio Code", "Code")));
        self.register(Arc::new(VSCodeManager::new("cursor", "Cursor", "cursor", "Cursor", "Cursor")));
        self.register(Arc::new(VSCodeManager::new("vscodium", "VSCodium", "codium", "VSCodium", "VSCodium")));
        self.register(Arc::new(VSCodeManager::new("roo", "Roo Cline", "roo", "Roo Code", "Roo Code")));
        self.register(Arc::new(VSCodeManager::new("windsurf", "Windsurf", "windsurf", "Windsurf", "Windsurf")));

        self.register(Arc::new(JetBrainsManager::new("idea", "IntelliJ IDEA", "idea")));
        self.register(Arc::new(JetBrainsManager::new("webstorm", "WebStorm", "webstorm")));
        self.register(Arc::new(JetBrainsManager::new("pycharm", "PyCharm", "pycharm")));
        self.register(Arc::new(JetBrainsManager::new("phpstorm", "PhpStorm", "phpstorm")));
        self.register(Arc::new(JetBrainsManager::new("rubymine", "RubyMine", "rubymine")));
        self.register(Arc::new(JetBrainsManager::new("goland", "GoLand", "goland")));
        self.register(Arc::new(JetBrainsManager::new("clion", "CLion", "clion")));
        self.register(Arc::new(JetBrainsManager::new("rider", "Rider", "rider")));
        self.register(Arc::new(JetBrainsManager::new("datagrip", "DataGrip", "datagrip")));
        self.register(Arc::new(JetBrainsManager::new("androidstudio", "Android Studio", "studio")));
        self.register(Arc::new(JetBrainsManager::new("fleet", "Fleet", "fleet")));

        self.register(Arc::new(VimManager::new()));
        self.register(Arc::new(NeovimManager::new()));
        self.register(Arc::new(EmacsManager::new()));

        #[cfg(target_os = "macos")]
        self.register(Arc::new(XcodeManager::new()));

        self.register(Arc::new(ZedManager::new()));
        self.register(Arc::new(SublimeManager::new()));
    }

    pub fn register(&self, manager: Arc<dyn EditorManager>) {
        let id = manager.id().to_string();
        self.managers.write().insert(id, manager);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn EditorManager>> {
        self.managers.read().get(id).cloned()
    }

    pub fn list_editors(&self) -> Vec<String> {
        self.managers.read().keys().cloned().collect()
    }
}

impl Default for EditorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
