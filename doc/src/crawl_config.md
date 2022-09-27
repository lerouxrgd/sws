# Crawler Config

The crawler configurable parameters are:

| Parameter      | Default                                                                                                                        | Description                                                                                                                                                                                                                      |
|----------------|--------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| user_agent     | "SWSbot"                                                                                                                       | The `User-Agent` header that will be used in all HTTP requests                                                                                                                                                                   |
| page_buffer    | 10_000                                                                                                                         | The size of the pages download queue. When the queue is full new downloads are on hold. This parameter is particularly relevant when using concurrent throttling.                                                                |
| throttle       | `Concurrent(100)` if `robot` is `None` <br><br>Otherwise `Delay(N)` where `N` is read from `robots.txt` field `Crawl-delay: N` | A throttling strategy for HTML pages download. <br><br>`Concurrent(N)` means at max `N` downloads at the same time, `PerSecond(N)` means at max `N` downloads per second, `Delay(N)` means wait for `N` seconds betwen downloads |
| num_workers    | max(1, num_cpus-2)                                                                                                             | The number of CPU cores that will be used for scraping page in parallel using the provided Lua script.                                                                                                                           |
| on_dl_error    | `SkipAndLog`                                                                                                                   | Behaviour when an error occurs while downloading an HTML page. Other possible value is `Fail`.                                                                                                                                   |
| on_xml_error   | `SkipAndLog`                                                                                                                   | Behaviour when an error occurs while processing a XML sitemap. Other possible value is `Fail`.                                                                                                                                   |
| on_scrap_error | `SkipAndLog`                                                                                                                   | Behaviour when an error occurs while scraping an HTML page in Lua. Other possible value is `Fail`.                                                                                                                               |
| robot          | `None`                                                                                                                         | An optional `robots.txt` URL used to retrieve a specific `Throttle::Delay`. <br><br>⚠ Conflicts with `seedRobotsTxt` in [Lua Scraper][lua-scraper], meaning that when `robot` is defined the `seed` cannot be a robot too. |

These parameters can be changed through Lua script or CLI arguments.

The priority order is: `CLI (highest priority) > Lua > Default values`

[lua-scraper]: ./lua_scraper.html#seed-definition

## Lua override

You can override parameters in Lua through the global variable `sws.crawlerConfig`.

| Parameter      | Lua name     | Example Lua value                   |
|----------------|--------------|-------------------------------------|
| user_agent     | userAgent    | "SWSbot"                            |
| page_buffer    | pageBuffer   | 10000                               |
| throttle       | throttle     | { Concurrent = 100 }                |
| num_workers    | numWorkers   | 4                                   |
| on_dl_error    | onDlError    | "SkipAndLog"                        |
| on_xml_error   | onXmlError   | "Fail"                              |
| on_scrap_error | onScrapError | "SkipAndLog"                        |
| robot          | robot        | "https://www.google.com/robots.txt" |


Here is an example of crawler configuration parmeters set using Lua:

```lua
-- You don't have to specify all parameters, only the ones you want to override.
sws.crawlerConfig = {
  userAgent = "SWSbot",
  pageBuffer = 10000,
  throttle = { Concurrent = 100 }, -- or: { PerSecond = 100 }, { Delay = 2 }
  numWorkers = 4,
  onDlError = "SkipAndLog", -- or: "Fail"
  onXmlError = "SkipAndLog",
  onScrapError = "SkipAndLog",
  robot = nil,
}
```

## CLI override

You can override parameters through the CLI arguments.

| Parameter            | CLI argument name | Example CLI argument value          |
|----------------------|-------------------|-------------------------------------|
| user_agent           | --user-agent      | 'SWSbot'                            |
| page_buffer          | --page-buffer     | 10000                               |
| throttle (Concurent) | --conc-dl         | 100                                 |
| throttle (PerSecond) | --rps             | 10                                  |
| throttle (Delay)     | --delay           | 2                                   |
| num_workers          | --num-workers     | 4                                   |
| on_dl_error          | --on-dl-error     | skip-and-log                        |
| on_xml_error         | --on-xml-error    | fail                                |
| on_scrap_error       | --on-scrap-error  | skip-and-log                        |
| robot                | --robot           | 'https://www.google.com/robots.txt' |

Here is an example of crawler configuration parmeters set using CLI arguments:

```sh
sws --script path/to/scrape_logic.lua -o results.csv     \
    --user-agent     'SWSbot'                            \
    --page-buffer    10000                               \
    --conc-dl        100                                 \
    --num-workers    4                                   \
    --on-dl-error    skip-and-log                        \
    --on-xml-error   fail                                \
    --on-scrap-error skip-and-log                        \
    --robot          'https://www.google.com/robots.txt' \
```
