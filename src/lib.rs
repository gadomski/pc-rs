use anyhow::{anyhow, Result};
use reqwest::{Client, IntoUrl};
use serde::Deserialize;
use stac::Asset;
use std::collections::HashMap;
use url::Url;

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
        if let Some(sas_request_url) = sas_request_url(&asset.href)? {
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

fn sas_request_url<U: IntoUrl>(url: U) -> Result<Option<String>> {
    let url = url.into_url()?;
    Container::maybe_parse(&url).map(|option| option.map(|container| container.token_url()))
}

fn sign(url: &str, token: &str) -> String {
    format!("{}?{}", url, token)
}

#[derive(Debug, Deserialize)]
struct Token {
    token: String,
}

#[derive(Debug)]
struct Container {
    account: String,
    name: String,
}

impl Container {
    fn maybe_parse(url: &Url) -> Result<Option<Container>> {
        if let Some(host_str) = url.host_str() {
            if is_nonpublic_azure_blob_storage_host(host_str) && !is_probably_signed(url) {
                let account = host_str
                    .split('.')
                    .next()
                    .ok_or_else(|| anyhow!("should be dots in the host string: {}", host_str))?;
                let name = url
                    .path()
                    .split("/")
                    .skip(1)
                    .next()
                    .ok_or_else(|| anyhow!("could not get container name from url: {}", url))?;
                Ok(Some(Container {
                    account: account.to_string(),
                    name: name.to_string(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn token_url(&self) -> String {
        format!("{}/{}/{}", TOKEN_URL, self.account, self.name)
    }
}

fn is_nonpublic_azure_blob_storage_host(host_str: &str) -> bool {
    host_str.ends_with(".blob.core.windows.net")
        && host_str != "ai4edatasetspublicassets.blob.core.windows.net"
}

fn is_probably_signed(url: &Url) -> bool {
    for (key, _) in url.query_pairs() {
        if key == "st" || key == "se" || key == "sp" {
            return true;
        }
    }
    false
}
