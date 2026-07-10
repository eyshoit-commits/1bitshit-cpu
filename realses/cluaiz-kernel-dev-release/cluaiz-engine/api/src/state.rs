use dispatcher::NeuralDispatcher;
use storage::EmbeddedManager;

/// Shared application state containing the dispatcher and storage manager.
pub struct AppState {
    pub dispatcher: NeuralDispatcher,
    pub embedded: EmbeddedManager,
}
