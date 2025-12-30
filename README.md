# Rustscape

A free-software, self-hosted replacement to the Geoscape API. Designed to provide information about properties and addresses in Australia.

This repo holds the various sub-projects for the project. For these are:

|Path|Description|
|---|---|
|fuzzbunny-rs|A simple reimplementation of [fuzzbunny](https://github.com/mixpanel/fuzzbunny/) in Rust, with some minor changes for use in Rustscape.|
|gnaf|Various scripts and tools to set up a GNAF database in Postgres from the raw GNAF files.|
|rustscape|The main server project.|

## Credits:

 - [fuzzbunny](https://github.com/mixpanel/fuzzbunny/) (MIT License, basis for fuzzy search algorithm)
 - [Geoscape Geocoded National Address File (G-NAF)](https://www.data.gov.au/data/dataset/geocoded-national-address-file-g-naf) - The underlying dataset that Rustscape utilises.

## License details:

- All files under `fuzzbunny-rs` are licensed under the [MIT license](fuzzbunny-rs/LICENSE).
- All files under `gnaf` are licensed under the [MIT license](gnaf/LICENSE).
- All files under `rustscape` are licensed under the [AGPLv3.0+ license(s)](rustscape/LICENSE) with an [exception for internal usage](rustscape/LICENSE-EXCEPTION).