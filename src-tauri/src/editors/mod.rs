mod jetbrains;
mod kate;
mod null;
mod others;
mod registry;
mod terminal;
mod traits;
mod vscode;

#[allow(unused_imports)] // Only used by integration tests
pub use null::NullEditor;
pub use registry::EditorRegistry;
pub use traits::OpenOptions;
