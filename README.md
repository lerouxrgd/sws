# Sitemap Web Scraper

Sitemap Web Scraper (sws) is a tool for simple, flexible, and yet performant web
pages scraping.

It consists of a CLI written in Rust that crawls web pages and executes a
[Lua JIT][lua-jit] script to scrap them, outputting results to a [CSV][] file.

```sh
sws crawl --script examples/fandom_mmh7.lua -o result.csv
```

Check out the [doc][sws-doc] for more details.

[lua-jit]: https://luajit.org/luajit.html
[csv]: https://en.wikipedia.org/wiki/Comma-separated_values
[sws-doc]: https://lerouxrgd.github.io/sws/