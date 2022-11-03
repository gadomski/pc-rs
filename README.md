# pcdownload

Download all of the assets from a Planetary Computer [STAC](https://stacspec.org/) [Item](https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md).

![Demo gif](docs/demo.gif)

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

To download to a specific directory:

```shell
pcdownload modis-17A2HGF-061 MYD17A2HGF.A2021361.h34v10.061.2022021015223 data
```

Use `--help` to see all options:

```shell
$ pcdownload --help
Usage: pcdownload <COLLECTION> <ID> [DIRECTORY]

Arguments:
  <COLLECTION>  STAC Collection id
  <ID>          STAC Item id
  [DIRECTORY]   Output directory. If not provided, use the current working directory

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

## License

**pcdownload** is dual-licensed under both the MIT license and the Apache license (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details.
