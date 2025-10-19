mod traits;
mod vscode;
mod jetbrains;
mod others;
mod terminal;
mod registry;

pub use traits::{EditorManager, OpenOptions, EditorInstance, EditorResult, EditorError};
pub use vscode::VSCodeManager;
pub use jetbrains::JetBrainsManager;
pub use others::SublimeManager;
pub use terminal::{VimManager, NeovimManager, EmacsManager};
pub use registry::EditorRegistry;
