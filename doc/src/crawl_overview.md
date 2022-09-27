# Subcommand: crawl

```text
Crawl sitemaps and scrap pages content

Usage: sws crawl [OPTIONS] --script <SCRIPT>

Options:
  -s, --script <SCRIPT>
          Path to the Lua script that defines scraping logic
  -o, --output-file <OUTPUT_FILE>
          Optional file that will contain scraped data, stdout otherwise
      --append
          Append to output file
      --truncate
          Truncate output file
  -q, --quiet
          Don't output logs
  -h, --help
          Print help information
```

More options in [CLI override](./crawl_config.md#cli-override)
