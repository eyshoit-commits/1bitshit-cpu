use strum::{Display, EnumIter, FromRepr};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Display, EnumIter, FromRepr)]
pub enum Tab {
    #[default]
    #[strum(to_string = " ALL ")]
    All,
    #[strum(to_string = " CHATS ")]
    Chat,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::All => Tab::Chat,
            Tab::Chat => Tab::All,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Tab::All => Tab::Chat,
            Tab::Chat => Tab::All,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Running,
    Quit,
}
