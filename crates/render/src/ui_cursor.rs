use bevy_window::SystemCursorIcon;

/// Cursor hint requested by a dioxus ui element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiCursor {
    /// Platform default arrow.
    #[default]
    Default,
    /// Clickable control.
    Pointer,
    /// Editable text.
    Text,
    /// Movable element.
    Move,
    /// Horizontal resize.
    ResizeHorizontal,
    /// Vertical resize.
    ResizeVertical,
    /// Diagonal resize toward the north west.
    ResizeNwse,
    /// Diagonal resize toward the north east.
    ResizeNesw,
}

impl UiCursor {
    /// Parses the shorthand written to the cursor attribute.
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "default" => Some(Self::Default),
            "pointer" => Some(Self::Pointer),
            "text" => Some(Self::Text),
            "move" => Some(Self::Move),
            "ew" => Some(Self::ResizeHorizontal),
            "ns" => Some(Self::ResizeVertical),
            "nwse" => Some(Self::ResizeNwse),
            "nesw" => Some(Self::ResizeNesw),
            _ => None,
        }
    }

    /// Returns the window cursor icon for this hint.
    pub fn system_icon(self) -> SystemCursorIcon {
        match self {
            Self::Default => SystemCursorIcon::Default,
            Self::Pointer => SystemCursorIcon::Pointer,
            Self::Text => SystemCursorIcon::Text,
            Self::Move => SystemCursorIcon::Move,
            Self::ResizeHorizontal => SystemCursorIcon::EwResize,
            Self::ResizeVertical => SystemCursorIcon::NsResize,
            Self::ResizeNwse => SystemCursorIcon::NwseResize,
            Self::ResizeNesw => SystemCursorIcon::NeswResize,
        }
    }
}
