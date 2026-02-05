//! DDNS module - Dynamic DNS update functionality

mod providers;
mod updater;

pub use self::providers::DdnsProviderTrait;
pub use self::updater::DdnsUpdater;
