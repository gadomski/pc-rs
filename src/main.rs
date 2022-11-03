use clap::Parser;
use indicatif::{MultiProgress, ProgressBar};
use reqwest::{Client, IntoUrl};
use serde::Deserialize;
use stac::Item;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

const API_URL: &str = "https://planetarycomputer.microsoft.com/api/stac/v1";
const TOKEN_URL: &str = "https://planetarycomputer.microsoft.com/api/sas/v1/token";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    collection: String,
    id: String,
    #[arg(short, long)]
    directory: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let url = format!(
        "{}/collections/{}/items/{}",
        API_URL, args.collection, args.id
    );
    let directory = args
        .directory
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let client = Client::new();

    println!("[1/3] Getting item...");
    let item: Item = client.get(url).send().await.unwrap().json().await.unwrap();

    println!("[2/3] Signing asset hrefs...");
    let mut urls = Vec::new();
    let mut cache = HashMap::new();
    for asset in item
        .assets
        .iter()
        .filter_map(|(key, value)| if key != "tilejson" { Some(value) } else { None })
    {
        if let Some(sas_request_url) = sas_request_url(&asset.href) {
            if cache.get(&sas_request_url).is_none() {
                let response = client.get(&sas_request_url).send().await.unwrap();
                let token: Token = response.json().await.unwrap();
                cache.insert(sas_request_url.clone(), token.token);
            }
            let url = sign(&asset.href, cache.get(&sas_request_url).unwrap());
            urls.push(url);
        }
    }

    println!("[3/3] Downloading assets...");
    let multi_progress = MultiProgress::new();
    let mut handles = Vec::new();
    std::fs::create_dir_all(&directory).unwrap();
    for url in urls {
        let progress_bar = multi_progress.add(ProgressBar::hidden());
        let path = directory.join(url.path_segments().unwrap().last().unwrap());
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let mut response = client.get(url).send().await.unwrap();
            if let Some(content_length) = response.content_length() {
                progress_bar.set_length(content_length);
            }
            let mut file = File::create(path).await.unwrap();
            while let Some(chunk) = response.chunk().await.unwrap() {
                progress_bar.inc(chunk.len() as u64);
                file.write_all(&chunk).await.unwrap();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
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
            for (key, value) in url.query_pairs() {
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

fn sign(url: &str, token: &str) -> Url {
    Url::parse(&format!("{}?{}", url, token)).unwrap()
}

#[derive(Debug, Deserialize)]
struct Token {
    token: String,
}
