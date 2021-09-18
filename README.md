# Twitter Image Downloader

## About

A commandline utillity to fetch all available images of a specific twitter user.

The images will be downloaded to a given folder in the format they are provided by twitter.

Due to a restriction of the Twitter API only the last 3600 tweets of the auther can be fetched.

## Prerequisites

In order to use this application you need to register as a twitter developer, and retrieve a set of tokens and secrets by creating a new "application" within your developer portal. This information is then used in order to authenticate against the twitter API used by this tool. All the keys need to be provided on the commandline for the tool to work.


## Usage

```shell
$ ./twitter_image_downloader --help

Twitter Image Downloader 1.0
Jakob Westhoff <jakob@westhoffswelt.de>
Download posted images from a given twitter user

USAGE:
    twitter_image_downloader [OPTIONS] <USERNAME> --access-token <TOKEN> --access-token-secret <SECRET> --consumer-key <KEY> --consumer-secret <SECRET>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -t, --access-token <TOKEN>            Twiter API Access Token
    -s, --access-token-secret <SECRET>    Twiter API Access Token Secret
    -k, --consumer-key <KEY>              Twiter API Consumer Key
    -c, --consumer-secret <SECRET>        Twiter API Consumer Secret
    -m, --max-requests <N>                Maximal number of parallel download requests [default: 4]
    -n, --max-images <N>                  Maximal number of images to download [default: 0]
    -o, --output-directory <DIRECTORY>    Directory to storage downloaded images in

ARGS:
    <USERNAME>    Twitter username to download images from.

```
