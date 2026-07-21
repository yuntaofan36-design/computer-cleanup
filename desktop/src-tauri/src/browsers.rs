use std::{
    fs::{self, Metadata},
    path::{Path, PathBuf},
};

const CHROMIUM_CACHE_DIRECTORIES: [(&str, &str); 3] = [
    ("Cache", "cache"),
    ("Code Cache", "code-cache"),
    ("GPUCache", "gpu-cache"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserDataRoot {
    Local,
    Roaming,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserProcess {
    Edge,
    Chrome,
    Brave,
    Vivaldi,
    Chromium,
    Opera,
    Yandex,
    QqBrowser,
    Qihoo360,
    Sogou,
    Maxthon,
    CentBrowser,
    Browser2345,
    Liebao,
    CocCoc,
    Arc,
    Browser115,
    UcBrowser,
    Firefox,
    Waterfox,
    Floorp,
    LibreWolf,
    Zen,
    PaleMoon,
}

impl BrowserProcess {
    pub fn names(self) -> &'static [&'static str] {
        match self {
            Self::Edge => &["msedge.exe", "msedge"],
            Self::Chrome => &["chrome.exe", "chrome"],
            Self::Brave => &["brave.exe", "brave"],
            Self::Vivaldi => &["vivaldi.exe", "vivaldi"],
            Self::Chromium => &["chromium.exe", "chromium"],
            Self::Opera => &["opera.exe", "opera"],
            Self::Yandex | Self::CocCoc => &["browser.exe", "browser"],
            Self::QqBrowser => &["qqbrowser.exe", "qqbrowser"],
            Self::Qihoo360 => &["360chrome.exe", "360chrome", "360se.exe", "360se"],
            Self::Sogou => &["sogouexplorer.exe", "sogouexplorer"],
            Self::Maxthon => &["maxthon.exe", "maxthon"],
            Self::CentBrowser => &["chrome.exe", "chrome"],
            Self::Browser2345 => &["2345explorer.exe", "2345explorer"],
            Self::Liebao => &["liebao.exe", "liebao"],
            Self::Arc => &["arc.exe", "arc"],
            Self::Browser115 => &["115chrome.exe", "115chrome"],
            Self::UcBrowser => &["ucbrowser.exe", "ucbrowser"],
            Self::Firefox => &["firefox.exe", "firefox"],
            Self::Waterfox => &["waterfox.exe", "waterfox"],
            Self::Floorp => &["floorp.exe", "floorp"],
            Self::LibreWolf => &["librewolf.exe", "librewolf"],
            Self::Zen => &["zen.exe", "zen"],
            Self::PaleMoon => &["palemoon.exe", "palemoon"],
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Edge => "Microsoft Edge",
            Self::Chrome => "Google Chrome",
            Self::Brave => "Brave",
            Self::Vivaldi => "Vivaldi",
            Self::Chromium => "Chromium",
            Self::Opera => "Opera",
            Self::Yandex => "Yandex Browser",
            Self::QqBrowser => "QQ 浏览器",
            Self::Qihoo360 => "360 浏览器",
            Self::Sogou => "搜狗高速浏览器",
            Self::Maxthon => "傲游浏览器",
            Self::CentBrowser => "百分浏览器",
            Self::Browser2345 => "2345 浏览器",
            Self::Liebao => "猎豹浏览器",
            Self::CocCoc => "Coc Coc",
            Self::Arc => "Arc",
            Self::Browser115 => "115 浏览器",
            Self::UcBrowser => "UC 浏览器",
            Self::Firefox => "Firefox",
            Self::Waterfox => "Waterfox",
            Self::Floorp => "Floorp",
            Self::LibreWolf => "LibreWolf",
            Self::Zen => "Zen Browser",
            Self::PaleMoon => "Pale Moon",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BrowserCacheRule {
    pub id: String,
    pub name: String,
    pub base: BrowserDataRoot,
    pub relative: PathBuf,
    pub process: BrowserProcess,
}

#[derive(Clone, Copy)]
enum BrowserLayout {
    ChromiumProfiles,
    DirectChromiumProfile,
    GeckoProfiles,
}

#[derive(Clone, Copy)]
struct BrowserSpec {
    id: &'static str,
    name: &'static str,
    base: BrowserDataRoot,
    relative: &'static str,
    layout: BrowserLayout,
    process: BrowserProcess,
}

const BROWSER_SPECS: &[BrowserSpec] = &[
    BrowserSpec {
        id: "edge",
        name: "Microsoft Edge",
        base: BrowserDataRoot::Local,
        relative: "Microsoft/Edge/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Edge,
    },
    BrowserSpec {
        id: "edge-beta",
        name: "Microsoft Edge Beta",
        base: BrowserDataRoot::Local,
        relative: "Microsoft/Edge Beta/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Edge,
    },
    BrowserSpec {
        id: "edge-dev",
        name: "Microsoft Edge Dev",
        base: BrowserDataRoot::Local,
        relative: "Microsoft/Edge Dev/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Edge,
    },
    BrowserSpec {
        id: "edge-canary",
        name: "Microsoft Edge Canary",
        base: BrowserDataRoot::Local,
        relative: "Microsoft/Edge SxS/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Edge,
    },
    BrowserSpec {
        id: "chrome",
        name: "Google Chrome",
        base: BrowserDataRoot::Local,
        relative: "Google/Chrome/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Chrome,
    },
    BrowserSpec {
        id: "chrome-beta",
        name: "Google Chrome Beta",
        base: BrowserDataRoot::Local,
        relative: "Google/Chrome Beta/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Chrome,
    },
    BrowserSpec {
        id: "chrome-dev",
        name: "Google Chrome Dev",
        base: BrowserDataRoot::Local,
        relative: "Google/Chrome Dev/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Chrome,
    },
    BrowserSpec {
        id: "chrome-canary",
        name: "Google Chrome Canary",
        base: BrowserDataRoot::Local,
        relative: "Google/Chrome SxS/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Chrome,
    },
    BrowserSpec {
        id: "brave",
        name: "Brave",
        base: BrowserDataRoot::Local,
        relative: "BraveSoftware/Brave-Browser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Brave,
    },
    BrowserSpec {
        id: "brave-beta",
        name: "Brave Beta",
        base: BrowserDataRoot::Local,
        relative: "BraveSoftware/Brave-Browser-Beta/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Brave,
    },
    BrowserSpec {
        id: "brave-nightly",
        name: "Brave Nightly",
        base: BrowserDataRoot::Local,
        relative: "BraveSoftware/Brave-Browser-Nightly/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Brave,
    },
    BrowserSpec {
        id: "vivaldi",
        name: "Vivaldi",
        base: BrowserDataRoot::Local,
        relative: "Vivaldi/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Vivaldi,
    },
    BrowserSpec {
        id: "chromium",
        name: "Chromium",
        base: BrowserDataRoot::Local,
        relative: "Chromium/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Chromium,
    },
    BrowserSpec {
        id: "yandex",
        name: "Yandex Browser",
        base: BrowserDataRoot::Local,
        relative: "Yandex/YandexBrowser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Yandex,
    },
    BrowserSpec {
        id: "qqbrowser",
        name: "QQ 浏览器",
        base: BrowserDataRoot::Local,
        relative: "Tencent/QQBrowser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::QqBrowser,
    },
    BrowserSpec {
        id: "360-extreme",
        name: "360 极速浏览器",
        base: BrowserDataRoot::Local,
        relative: "360Chrome/Chrome/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Qihoo360,
    },
    BrowserSpec {
        id: "360-safe-local",
        name: "360 安全浏览器",
        base: BrowserDataRoot::Local,
        relative: "360se6/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Qihoo360,
    },
    BrowserSpec {
        id: "360-safe-roaming",
        name: "360 安全浏览器",
        base: BrowserDataRoot::Roaming,
        relative: "360se6/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Qihoo360,
    },
    BrowserSpec {
        id: "sogou-local",
        name: "搜狗高速浏览器",
        base: BrowserDataRoot::Local,
        relative: "SogouExplorer/Webkit",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Sogou,
    },
    BrowserSpec {
        id: "sogou-roaming",
        name: "搜狗高速浏览器",
        base: BrowserDataRoot::Roaming,
        relative: "SogouExplorer/Webkit",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Sogou,
    },
    BrowserSpec {
        id: "maxthon",
        name: "傲游浏览器",
        base: BrowserDataRoot::Local,
        relative: "Maxthon/Application/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Maxthon,
    },
    BrowserSpec {
        id: "centbrowser",
        name: "百分浏览器",
        base: BrowserDataRoot::Local,
        relative: "CentBrowser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::CentBrowser,
    },
    BrowserSpec {
        id: "2345explorer",
        name: "2345 浏览器",
        base: BrowserDataRoot::Local,
        relative: "2345Explorer/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Browser2345,
    },
    BrowserSpec {
        id: "liebao",
        name: "猎豹浏览器",
        base: BrowserDataRoot::Local,
        relative: "liebao/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Liebao,
    },
    BrowserSpec {
        id: "coccoc",
        name: "Coc Coc",
        base: BrowserDataRoot::Local,
        relative: "CocCoc/Browser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::CocCoc,
    },
    BrowserSpec {
        id: "arc",
        name: "Arc",
        base: BrowserDataRoot::Local,
        relative: "Packages/TheBrowserCompany.Arc_ttt1ap7aakyb4/LocalCache/Local/Arc/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Arc,
    },
    BrowserSpec {
        id: "115browser",
        name: "115 浏览器",
        base: BrowserDataRoot::Local,
        relative: "115Browser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::Browser115,
    },
    BrowserSpec {
        id: "ucbrowser",
        name: "UC 浏览器",
        base: BrowserDataRoot::Local,
        relative: "UCBrowser/User Data",
        layout: BrowserLayout::ChromiumProfiles,
        process: BrowserProcess::UcBrowser,
    },
    BrowserSpec {
        id: "opera",
        name: "Opera",
        base: BrowserDataRoot::Local,
        relative: "Opera Software/Opera Stable",
        layout: BrowserLayout::DirectChromiumProfile,
        process: BrowserProcess::Opera,
    },
    BrowserSpec {
        id: "opera-gx",
        name: "Opera GX",
        base: BrowserDataRoot::Local,
        relative: "Opera Software/Opera GX Stable",
        layout: BrowserLayout::DirectChromiumProfile,
        process: BrowserProcess::Opera,
    },
    BrowserSpec {
        id: "opera-air",
        name: "Opera Air",
        base: BrowserDataRoot::Local,
        relative: "Opera Software/Opera Air Stable",
        layout: BrowserLayout::DirectChromiumProfile,
        process: BrowserProcess::Opera,
    },
    BrowserSpec {
        id: "opera-developer",
        name: "Opera Developer",
        base: BrowserDataRoot::Local,
        relative: "Opera Software/Opera Developer",
        layout: BrowserLayout::DirectChromiumProfile,
        process: BrowserProcess::Opera,
    },
    BrowserSpec {
        id: "opera-beta",
        name: "Opera Beta",
        base: BrowserDataRoot::Local,
        relative: "Opera Software/Opera Next",
        layout: BrowserLayout::DirectChromiumProfile,
        process: BrowserProcess::Opera,
    },
    BrowserSpec {
        id: "firefox",
        name: "Firefox",
        base: BrowserDataRoot::Local,
        relative: "Mozilla/Firefox/Profiles",
        layout: BrowserLayout::GeckoProfiles,
        process: BrowserProcess::Firefox,
    },
    BrowserSpec {
        id: "waterfox",
        name: "Waterfox",
        base: BrowserDataRoot::Local,
        relative: "Waterfox/Profiles",
        layout: BrowserLayout::GeckoProfiles,
        process: BrowserProcess::Waterfox,
    },
    BrowserSpec {
        id: "floorp",
        name: "Floorp",
        base: BrowserDataRoot::Local,
        relative: "Floorp/Profiles",
        layout: BrowserLayout::GeckoProfiles,
        process: BrowserProcess::Floorp,
    },
    BrowserSpec {
        id: "librewolf",
        name: "LibreWolf",
        base: BrowserDataRoot::Local,
        relative: "librewolf/Profiles",
        layout: BrowserLayout::GeckoProfiles,
        process: BrowserProcess::LibreWolf,
    },
    BrowserSpec {
        id: "zen",
        name: "Zen Browser",
        base: BrowserDataRoot::Local,
        relative: "zen/Profiles",
        layout: BrowserLayout::GeckoProfiles,
        process: BrowserProcess::Zen,
    },
    BrowserSpec {
        id: "palemoon",
        name: "Pale Moon",
        base: BrowserDataRoot::Local,
        relative: "Moonchild Productions/Pale Moon/Profiles",
        layout: BrowserLayout::GeckoProfiles,
        process: BrowserProcess::PaleMoon,
    },
];

fn is_link_or_reparse(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        return metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    #[cfg(not(windows))]
    false
}

fn trusted_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !is_link_or_reparse(&metadata))
        .unwrap_or(false)
}

fn child_directory_names(parent: &Path) -> Vec<String> {
    if !trusted_directory(parent) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut names = entries
        .flatten()
        .filter(|entry| trusted_directory(&entry.path()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    names.sort_by_cached_key(|name| name.to_ascii_lowercase());
    names
}

fn stable_id_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn chromium_profile_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("Default")
        || name
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Profile "))
            && name.len() > 8
}

fn add_chromium_cache_rules(
    rules: &mut Vec<BrowserCacheRule>,
    spec: BrowserSpec,
    base_root: &Path,
    profile_relative: &Path,
    profile_name: &str,
) {
    let profile_id = stable_id_component(profile_name);
    for (cache_directory, cache_id) in CHROMIUM_CACHE_DIRECTORIES {
        let relative = profile_relative.join(cache_directory);
        if !trusted_directory(&base_root.join(&relative)) {
            continue;
        }
        rules.push(BrowserCacheRule {
            id: format!("browser-{}-profile-{profile_id}-{cache_id}", spec.id),
            name: format!("{} · {} · {cache_directory}", spec.name, profile_name),
            base: spec.base,
            relative,
            process: spec.process,
        });
    }
}

fn discover_for_spec(rules: &mut Vec<BrowserCacheRule>, spec: BrowserSpec, base_root: &Path) {
    let data_relative = Path::new(spec.relative);
    let data_root = base_root.join(data_relative);
    if !trusted_directory(&data_root) {
        return;
    }

    match spec.layout {
        BrowserLayout::ChromiumProfiles => {
            for profile in child_directory_names(&data_root)
                .into_iter()
                .filter(|profile| chromium_profile_name(profile))
            {
                add_chromium_cache_rules(
                    rules,
                    spec,
                    base_root,
                    &data_relative.join(&profile),
                    &profile,
                );
            }
        }
        BrowserLayout::DirectChromiumProfile => {
            add_chromium_cache_rules(rules, spec, base_root, data_relative, "默认配置");
        }
        BrowserLayout::GeckoProfiles => {
            for profile in child_directory_names(&data_root) {
                let relative = data_relative.join(&profile).join("cache2");
                if !trusted_directory(&base_root.join(&relative)) {
                    continue;
                }
                rules.push(BrowserCacheRule {
                    id: format!(
                        "browser-{}-profile-{}-cache2",
                        spec.id,
                        stable_id_component(&profile)
                    ),
                    name: format!("{} · {profile} · cache2", spec.name),
                    base: spec.base,
                    relative,
                    process: spec.process,
                });
            }
        }
    }
}

pub fn discover_cache_rules(
    local_root: Option<&Path>,
    roaming_root: Option<&Path>,
) -> Vec<BrowserCacheRule> {
    let mut rules = Vec::new();
    for spec in BROWSER_SPECS {
        let base_root = match spec.base {
            BrowserDataRoot::Local => local_root,
            BrowserDataRoot::Roaming => roaming_root,
        };
        let Some(base_root) = base_root else {
            continue;
        };
        discover_for_spec(&mut rules, *spec, base_root);
    }
    rules
}
