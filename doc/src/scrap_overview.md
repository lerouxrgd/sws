# Subcommand: scrap

```text
Scrap a single remote page or multiple local pages

Usage: sws scrap [OPTIONS] --script <SCRIPT> <--url <URL>|--files <GLOB>>

Options:
  -s, --script <SCRIPT>            Path to the Lua script that defines scraping logic
      --url <URL>                  A distant html page to scrap
      --files <GLOB>               A glob pattern to select local files to scrap
  -o, --output-file <OUTPUT_FILE>  Optional file that will contain scraped data, stdout otherwise
      --append                     Append to output file
      --truncate                   Truncate output file
      --num-workers <NUM_WORKERS>  Set the number of CPU workers when scraping local files
      --on-error <ON_ERROR>        Scrap error handling strategy when scraping local files [possible values: fail, skip-and-log]
  -q, --quiet                      Don't output logs
  -h, --help                       Print help information
```

The parameters `--url` and `--files` are mutually exclusive (only one can be specified).

This subcommand is meant to either:

* Quickly test a [Lua script](./lua_scraper.html) on a given URL (with `--url`)

* Process HTML pages that have been previously stored on disk (with `--files`)
