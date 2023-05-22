use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Release {
    pub html_url: String,
    pub id: i64,
    pub tag_name: String,
    pub name: Option<String>,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub url: String,
    pub browser_download_url: String,
    pub id: i64,
    pub name: String,
    pub content_type: String,
    pub size: usize,
}
