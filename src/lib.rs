use nomanga_sdk::extension::{info::ExtensionInfo, prelude::ABI_VERSION};

pub mod sources;

nomanga_sdk::register_sources! {
    extension: ExtensionInfo {
        id: "dev.yuri.nsfwpack".into(),
        name: "NSFW Pack".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        abi_version: ABI_VERSION,
        author: "Yuri".into(),
        website: None },
    sources: [sources::nhentai::NHentaiSource, sources::hitomi::HitomiSource],
}
