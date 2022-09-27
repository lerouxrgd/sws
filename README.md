# Sitemap Web Scraper

Sitemap Web Scraper, or [sws][], is a tool for simple, flexible, and yet performant web
pages scraping.

It consists of a [CLI][] written in [Rust][] that crawls web pages and executes a
[Lua][] [JIT][lua-jit] script to scrap them, outputting results to a [CSV][] file.

```sh
sws crawl --script examples/fandom_mmh7.lua -o result.csv
```

Check out the [doc][sws] for more details.

[sws]: https://lerouxrgd.github.io/sws/
[cli]: https://en.wikipedia.org/wiki/Command-line_interface
[rust]: https://www.rust-lang.org/
[lua]: https://www.lua.org/
[lua-jit]: https://luajit.org/
[csv]: https://en.wikipedia.org/wiki/Comma-separated_values
