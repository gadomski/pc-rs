use anyhow::{Error, Result};
use clap::Parser;
use console::{style, Emoji};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use path_slash::PathBufExt;
use planetary_computer::SasCache;
use reqwest::Client;
use stac::{media_type::GEOJSON, Item, Link};
use std::{collections::HashMap, path::PathBuf};
use tokio::{fs::File, io::AsyncWriteExt};
use url::Url;

const SMALL_BLUE_DIAMOND: Emoji<'_, '_> = Emoji("🔹 ", "");
const WRITING_HAND: Emoji<'_, '_> = Emoji("✍️️  ", "");
const ENVELOPE_WITH_ARROW: Emoji<'_, '_> = Emoji("📩 ", "");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// STAC Collection id
    collection: String,

    /// STAC Item id
    id: String,

    /// Output directory. If not provided, use the current working directory.
    directory: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let spinner_style =
        ProgressStyle::with_template("{prefix:.bold.dim} {spinner} [{elapsed}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
    let args = Args::parse();
    let item_url = planetary_computer::item_url(&args.collection, &args.id);
    let directory = args
        .directory
        .map(Ok)
        .or_else(|| Some(std::env::current_dir()))
        .transpose()?
        .unwrap();
    let client = Client::new();

    println!(
        "{} {}Getting item...",
        style("[1/3]").bold().dim(),
        SMALL_BLUE_DIAMOND
    );
    let mut item: Item = client
        .get(&item_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!(
        "{} {}Signing asset hrefs...",
        style("[2/3]").bold().dim(),
        WRITING_HAND
    );
    let mut sas_cache = SasCache::with_client(client.clone());
    let mut assets = HashMap::new();
    for (key, asset) in item.assets.drain().filter(|(key, _)| key != "tilejson") {
        assets.insert(key, sas_cache.sign_asset(asset).await?);
    }

    println!(
        "{} {}Downloading assets...",
        style("[3/3]").bold().dim(),
        ENVELOPE_WITH_ARROW
    );
    let multi_progress = MultiProgress::new();
    let mut handles = Vec::new();
    std::fs::create_dir_all(&directory)?;
    let directory = directory.canonicalize()?;
    let num_assets = assets.len();
    for (i, (key, mut asset)) in assets.into_iter().enumerate() {
        let progress_bar = multi_progress.add(ProgressBar::hidden());
        let spinner_style = spinner_style.clone();
        let url = Url::parse(&asset.href)?;
        let file_name = url.path_segments().unwrap().last().unwrap().to_string();
        let path = directory.join(&file_name);
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let mut response = match client.get(url).send().await {
                Ok(response) => response.error_for_status()?,
                Err(err) => return Err(Error::from(err)),
            };
            if let Some(content_length) = response.content_length() {
                progress_bar.set_length(content_length);
            }
            progress_bar.set_style(spinner_style);
            progress_bar.set_prefix(format!("[{}/{}]", i + 1, num_assets));
            progress_bar.set_message(format!("{}", path.file_name().unwrap().to_string_lossy()));
            let mut file = File::create(path).await?;
            while let Some(chunk) = response.chunk().await? {
                progress_bar.inc(chunk.len() as u64);
                file.write_all(&chunk).await?;
            }
            asset.href = format!("./{}", file_name);
            Ok((key, asset))
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await.unwrap() {
            Ok((key, asset)) => {
                let _ = item.assets.insert(key, asset);
            }
            Err(err) => {
                eprintln!("{}: {}", style("Error when downloading asset").red(), err)
            }
        }
    }
    let href = directory.join(format!("{}.json", item.id));
    item.links.retain(|link| !link.is_self());
    item.links.push(Link {
        href: href.to_slash().unwrap().into_owned(),
        rel: "self".to_string(),
        r#type: Some(GEOJSON.to_string()),
        title: None,
        additional_fields: Default::default(),
    });
    item.links.push(Link {
        href: item_url,
        rel: "canonical".to_string(),
        r#type: Some(GEOJSON.to_string()),
        title: None,
        additional_fields: Default::default(),
    });
    let item = serde_json::to_vec_pretty(&item)?;
    let mut file = File::create(href).await?;
    file.write_all(&item).await?;

    Ok(())
}
