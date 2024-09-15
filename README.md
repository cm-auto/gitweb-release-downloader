# gitweb-release-downloader

Allows you to download release assets from GitHub and Gitea (thus Forgejo is
supported, too).\
Additionally you can query a repository's releases and their respective assets.\
Support for GitLab is planned.

## Usage

Downloading VSCodium

```bash
grd download "github.com/VSCodium/vscodium" "\\.deb$"
```

Alternatively

```bash
grd download --website-type github "VSCodium/vscodium" "\\.deb$"
```

First argument is the repository\
Second argument is a regex pattern for the asset to download\
`--website-type` takes the type of git website (if this is omitted, the program
tries to guess it from the passed repository)

Downloading from the latest release of Forgejo on codeberg.org:

```bash
grd download --website-type gitea codeberg.org/forgejo/forgejo ".*"
```

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

To query releases of a repository

```bash
grd query releases "github.com/VSCodium/vscodium"
```

By default it will only print the latest release, which is not a prerelease.\
You can change this with the `--count` and `--prerelease` flag.

To query assets of a repository

```bash
grd query assets "github.com/VSCodium/vscodium"
```

By default this will print all assets of the latest release, which is not a
prerelease.\
To query from a specific release you can use the `--tag` flag (including
prereleases).\
Limiting the assets to show is done with the `--asset-pattern` flag.
