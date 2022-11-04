use anyhow::Result;
use reqwest::{Client, IntoUrl};
use serde::Deserialize;
use stac::Asset;
use std::collections::HashMap;

const API_URL: &str = "https://planetarycomputer.microsoft.com/api/stac/v1";
const TOKEN_URL: &str = "https://planetarycomputer.microsoft.com/api/sas/v1/token";

pub fn item_url(collection_id: &str, item_id: &str) -> String {
    format!(
        "{}/collections/{}/items/{}",
        API_URL, collection_id, item_id
    )
}

pub fn collection_url(collection_id: &str) -> String {
    format!("{}/collections/{}", API_URL, collection_id)
}

pub struct SasCache {
    cache: HashMap<String, String>,
    client: Client,
}

impl SasCache {
    pub fn with_client(client: Client) -> SasCache {
        SasCache {
            cache: HashMap::new(),
            client,
        }
    }
    pub async fn sign_asset(&mut self, mut asset: Asset) -> Result<Asset> {
        if let Some(sas_request_url) = sas_request_url(&asset.href) {
            if self.cache.get(&sas_request_url).is_none() {
                let response = self
                    .client
                    .get(&sas_request_url)
                    .send()
                    .await?
                    .error_for_status()?;
                let token: Token = response.json().await?;
                self.cache.insert(sas_request_url.clone(), token.token);
            }
            asset.href = sign(&asset.href, self.cache.get(&sas_request_url).unwrap()).to_string();
        }
        Ok(asset)
    }
}

fn sas_request_url<U: IntoUrl>(url: U) -> Option<String> {
    let url = url.into_url().unwrap();
    if let Some(host_str) = url.host_str() {
        if !host_str.ends_with(".blob.core.windows.net")
            || host_str == "ai4edatasetspublicassets.blob.core.windows.net"
        {
            None
        } else {
            for (key, _) in url.query_pairs() {
                if key == "st" || key == "se" || key == "sp" {
                    return None;
                }
            }
            let account_name = host_str.split('.').next().unwrap();
            let container_name = url.path().split("/").skip(1).next().unwrap();
            Some(format!("{}/{}/{}", TOKEN_URL, account_name, container_name))
        }
    } else {
        None
    }
}

fn sign(url: &str, token: &str) -> String {
    format!("{}?{}", url, token)
}

#[derive(Debug, Deserialize)]
struct Token {
    token: String,
}
