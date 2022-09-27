# Introduction

Sitemap Web Scraper, or [sws][], is a tool for simple, flexible, and yet performant web
pages scraping. It consists of a [CLI][] that executes a [Lua][] [JIT][lua-jit] script
and outputs a [CSV][] file.

All the logic for crawling/scraping is defined in Lua and executed on a multiple threads
in [Rust][]. The actual parsing of HTML is done in Rust. Standard [CSS
selectors][css-sel] are also implemented in Rust (using Servo's [html5ever][] and
[selectors][]). Both functionalities are accessible through a Lua API for flexible
scraping logic.

As for the crawling logic, multiple seeding options are available: [robots.txt][robots],
[sitemaps][], or a custom HTML pages list. By default, sitemaps (either provided or
extracted from `robots.txt`) will be crawled recursively and the discovered HTML pages
will be scraped with the provided Lua script. It's also possible to dynamically add page
links to the crawling queue when scraping an HTML page. See the [crawl][sub-crawl]
subcommand and the [Lua scraper][lua-scraper] for more details.

Besides, the Lua scraping script can be used on HTML pages stored as local files,
without any crawling. See the [scrap][sub-scrap] subcommand doc for more details.

Furthermore, the CLI is composed of `crates` that can be used independently in a custom
Rust program.

[sws]: https://github.com/lerouxrgd/sws
[cli]: https://en.wikipedia.org/wiki/Command-line_interface
[rust]: https://www.rust-lang.org/
[lua]: https://www.lua.org/
[lua-jit]: https://luajit.org/
[csv]: https://en.wikipedia.org/wiki/Comma-separated_values
[css-sel]: https://www.w3schools.com/cssref/css_selectors.asp
[html5ever]: https://crates.io/crates/html5ever
[selectors]: https://crates.io/crates/selectors
[robots]: https://en.wikipedia.org/wiki/Robots.txt
[sitemaps]: https://www.sitemaps.org/
[sub-crawl]: ./crawl_overview.html
[sub-scrap]: ./scrap_overview.html
[lua-scraper]: ./lua_scraper.html
