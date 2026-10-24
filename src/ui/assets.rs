//! Use Kit's complete icon source so every application and component icon is available.
use gpui_kit::AssetSource;
pub use gpui_kit::assets::IconName;

pub fn source() -> impl AssetSource {
    ely::Assets::before(gpui_kit::assets::AllAssets)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_and_component_icons_are_available() {
        let assets = source();
        // Ely's initialization validates its assets against this same combined source.
        for path in ely::Assets.list("").unwrap() {
            assert!(
                assets.load(&path).unwrap().is_some(),
                "Ely 资源缺失：{path}"
            );
        }

        for icon in [
            IconName::Github,
            IconName::Eraser,
            IconName::Clock,
            IconName::Binary,
            IconName::Languages,
            IconName::Hash,
            IconName::Shield,
            IconName::Braces,
            IconName::Link,
            IconName::QrCode,
            IconName::Music,
            IconName::Image,
            IconName::FileMusic,
            IconName::FileVideoCamera,
            IconName::FolderOpen,
            IconName::ChevronRight,
            IconName::WindowMinimize,
            IconName::WindowMaximize,
            IconName::WindowRestore,
            IconName::WindowClose,
            IconName::Calendar,
        ] {
            let path = icon.path();
            let bytes = assets.load(&path).unwrap().expect("图标资源缺失");
            assert!(
                std::str::from_utf8(&bytes).unwrap().contains("<svg"),
                "无效图标：{path}"
            );
        }
    }
}
