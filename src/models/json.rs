use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub browser_download_url: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GitLabRelease {
    pub tag_name: String,
    pub upcoming_release: bool,
    pub assets: GitLabAssets,
}

impl From<GitLabRelease> for Release {
    fn from(value: GitLabRelease) -> Self {
        Self {
            tag_name: value.tag_name,
            prerelease: value.upcoming_release,
            assets: value.assets.links.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GitLabAssets {
    pub links: Vec<GitLabAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitLabAsset {
    pub name: String,
    pub direct_asset_url: String,
}

impl From<GitLabAsset> for Asset {
    fn from(value: GitLabAsset) -> Self {
        Self {
            browser_download_url: value.direct_asset_url,
            name: value.name,
        }
    }
}
