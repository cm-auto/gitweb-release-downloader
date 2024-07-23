# gitweb-release-downloader

Allows you to download release assets from (currently only) GitHub.

## Usage

Downloading VSCodium

```bash
grd download "github.com/VSCodium/vscodium" "\\.deb$"
```

Alternatively

```bash
grd download --website-type github "VSCodium/vscodium" "\\.deb$"
```

`--repository` takes the owner and name of the repository\
`--website-type` takes the type of git website (if this is omitted, the program
tries to guess it from the passed repository)\
`--asset-pattern` takes a regex pattern for the asset to download

It automatically takes the latest release, which is not a prerelease.\
Alternatively it takes a tag to download with `--tag`.\
If you want to allow prereleases add `--prerelease`.

You can also make the program print the downloaded file name and pipe it to
another program or save it in a variable.\
This for example allows automatic installation:

```bash
filename=$(grd download "github.com/VSCodium/vscodium" "\\.deb$" --print-filename)
sudo apt install "./$filename" && rm "$filename"
```
