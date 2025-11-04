mod emacs;
mod kakoune;
mod micro;
mod nano;
mod neovim;
mod terminal_detector;
mod vim;

pub use emacs::EmacsManager;
pub use kakoune::KakouneManager;
pub use micro::MicroManager;
pub use nano::NanoManager;
pub use neovim::NeovimManager;
pub use vim::VimManager;
