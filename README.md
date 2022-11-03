# pcdownload

Download all of the assets from a Planetary Computer [STAC](https://stacspec.org/) [Item](https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md).

## Installation

Install rust, e.g. with [rustup](https://rustup.rs/).
Then, install **pcdownload**:

```shell
cargo install --git https://github.com/gadomski/pcdownload
```

## Usage

To download the assets to your current working directory:

```shell
pcdownload modis-17A2HGF-061 MYD17A2HGF.A2021361.h34v10.061.2022021015223
```

You can use the `--directory` option to specify the output directory.

## License

**pcdownload** is dual-licensed under both the MIT license and the Apache license (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details.
