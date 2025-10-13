use std::fmt::Display;

// pub mod asset_handle;
pub mod asset_single;
pub mod component_single;
pub mod resource;

pub mod traits;


/// What dioxus shows incase the unerlying can't be fetched.
pub enum BevyFetchBackup {
    /// Return value as unknown as it couldn't be fetched
    Unknown,
    /// Return lorem ipsum block
    LoremIpsum,
    /// Return value for when the value exists in bevy, but dioxus hasn't received it yet.
    Uninitialized,
}

impl Default for BevyFetchBackup {
    fn default() -> Self {
        BevyFetchBackup::Unknown
    }
}

impl Display for BevyFetchBackup{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            BevyFetchBackup::Unknown => "Unable to receive value",
            // todo: Implement this properly.
            BevyFetchBackup::LoremIpsum => "Lorem Ipsum",
            BevyFetchBackup::Uninitialized => "waiting for value from bevy....",
        };
        write!(f, "{}", string)
    }
}