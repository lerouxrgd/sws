# Lua API Overview

<style>
  table {
    width: 100%;
  }
  th {
    white-space: nowrap;
    text-align: left;
  }
  td:nth-child(1) {
    white-space: nowrap;
  }
</style>

## Global variables

| Lua name  | Lua Type | Description                                                                                                                                          |
|-----------|----------|------------------------------------------------------------------------------------------------------------------------------------------------------|
| scrapPage | function | Define the scraping logic for a single HTML page. See [details](./lua_scraper.html#function-accepturl)                                               |
| acceptUrl | function | Specify whether to accept a URL when crawling an [XML Sitemap][xml-sitemap], `true` by default. See [details](./lua_scraper.html#function-scrappage) |
| sws       | table    | The sws namespace                                                                                                                                    |

[xml-sitemap]: https://en.wikipedia.org/wiki/Site_map

## Namespaced variables

All the following variables are defined in the `sws` table.

### Seeds

The configurable [seed](./lua_scraper.html#seed-definition)

| Lua name      | Lua Type | Description              |
|---------------|----------|--------------------------|
| seedSitemaps  | table    | A list of sitemap URLs   |
| seedPages     | table    | A list of HTML page URLs |
| seedRobotsTxt | string   | A single robots.txt URL  |

### Configurations

| Lua name        | Lua Type | Description                                                                           |
|-----------------|----------|---------------------------------------------------------------------------------------|
| csvWriterConfig | table    | Config used to write output csv records. See [details](./lua_scraper.html#csv-record) |
| crawlerConfig   | table    | Config used to customize crawler behavior. See [details](./crawl_config.html)         |

## Types

All types are defined in the `sws` table.

### Class Html

A parsed HTML page. Its HTML elements can be selected with [CSS selectors](./lua_scraper.html#css-selectors).

| Lua signature                           | Description                                                                    |
|-----------------------------------------|--------------------------------------------------------------------------------|
| Html:select(selector: string) -> Select | Parses the given CSS `selector` and returns a [Select](#class-select) instance |
| Html:root() -> ElemRef                  | Returns an [ElemRef](#class-elemref) to the HTML root node                     |

### Class Select

A selection made with [CSS selectors](./lua_scraper.html#css-selectors). Its HTML elements can be iterated.

| Lua signature                                      | Description                                                                             |
|----------------------------------------------------|-----------------------------------------------------------------------------------------|
| Select:iter() -> iterator&lt;ElemRef&gt;           | An iterator of [ElemRef](#class-elemref) over the selected HTML nodes                   |
| Select:enumerate() -> iterator<(integer, ElemRef)> | An iterator of [ElemRef](#class-elemref) and their indices over the selected HTML nodes |

### Class ElemRef

An HTML element reference. Its descendant HTML elements can be selected with [CSS selectors](./lua_scraper.html#css-selectors).

| Lua signature                              | Description                                                                                         |
|--------------------------------------------|-----------------------------------------------------------------------------------------------------|
| ElemRef:select(selector: string) -> Select | Parses the given CSS `selector` and returns a [Select](#class-select) instance over its descendants |
| ElemRef:innerHtml() -> string              | The inner HTML string of this element                                                               |
| ElemRef:innerText() -> string              | Returns all the descendent text nodes content concatenated                                          |
| ElemRef:name() -> string                   | The HTML element name                                                                               |
| ElemRef:id() -> string                     | The HTML element id, if any                                                                         |
| ElemRef:hasClass(class: string) -> boolean | Whether the HTML element has the given `class`                                                      |
| ElemRef:classes() -> table                 | Returns all classes of the HTML element                                                             |
| ElemRef:attr(name: string) -> string       | If the HTML element has the `name` attribute, return its value, nil otherwise                       |
| ElemRef:attrs() -> table                   | Returns all attributes of the HTML element                                                          |

### Class Date

A helper class for parsing and formatting dates.

| Lua signature                           | Description                                                                                                         |
|-----------------------------------------|---------------------------------------------------------------------------------------------------------------------|
| Date(date: string, fmt: string) -> Date | Parses the given `date` accordingly to `fmt`, uses [chrono::NaiveDate::parse_from_str][chrono-parse] under the hood |
| Date:format(fmt: string) -> string      | Formats the current date accordingly to `fmt`, uses [chrono::NaiveDate::format][chrono-format] under the hood       |

[chrono-parse]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html#method.parse_from_str
[chrono-format]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html#method.format

### Class ScrapingContext

The context available when an HTML page is scraped, provided as parameter in [scrapPage](./lua_scraper.html#function-scrappage)

| Lua signature                                  | Description                                                                                                 |
|------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| ScrapingContext:pageLocation() -> PageLocation | Returns the current [PageLocation](#class-pagelocation)                                                     |
| ScrapingContext:sendRecord(rec: Record)        | Sends a CSV [Record](#class-record) to the current output (either `stdout` or the specified output file)    |
| ScrapingContext:sendUrl(url: string)           | Adds the given `url` to the internal crawling queue so that it will be scraped later                        |
| ScrapingContext:workerId() -> string           | A string identifying the current worker thread. It simply consists of the worker's number (starting from 0) |
| ScrapingContext:robot() -> Robot               | Returns current [Robot](#class-robot) if it was [setup](./lua_scraper.html#robot-definition), nil otherwise |

### Class PageLocation

The location of an HTML page.

| Lua signature                                 | Description                                                                                                 |
|-----------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| PageLocation:kind() -> option&lt;Location&gt; | Get the page's [Location](#enum-location) kind                                                              |
| PageLocation:get() -> option&lt;string&gt;    | If the current page is a `Location.URL` returns its URL, if it's a `Location.PATH` returns its path on disk |

### Enum Location

Location kind.

| Lua variant   | Description                                                                                     |
|---------------|-------------------------------------------------------------------------------------------------|
| Location.URL  | A URL location kind (remote). Relevant when using the [crawl subcommand](./crawl_overview.html) |
| Location.PATH | A PATH location kind (local). Relevant when using the [scrap subcommand](./scrap_overview.html) |

### Class Record

A dynamic CSV record. CSV formatting can be customized (see [details](./lua_scraper.html#csv-record)).

| Lua signature                   | Description                                     |
|---------------------------------|-------------------------------------------------|
| Record() -> Record              | Creates a new empty CSV record                  |
| Record:pushField(field: string) | Adds the given `field` value to this CSV record |

### Class CrawlingContext

The context available when an XML Sitemap page is crawled, provided as parameter in [acceptUrl](./lua_scraper.html#function-accepturl)

| Lua signature                        | Description                                                                                                 |
|--------------------------------------|-------------------------------------------------------------------------------------------------------------|
| CrawlingContext:robot() -> Robot     | Returns current [Robot](#class-robot) if it was [setup](./lua_scraper.html#robot-definition), nil otherwise |
| CrawlingContext:sitemap() -> Sitemap | The [Sitemap](#enum-sitemap) format of the sitemap page being crawled                                       |

### Class Robot

| Lua signature                         | Description                                                                                                            |
|---------------------------------------|------------------------------------------------------------------------------------------------------------------------|
| Robot:allowed(url: string) -> boolean | Whether the given `url` is allowed for crawling or not. This relies on [texting_robots::Robot::allowed][robot-allowed] |

[robot-allowed]: https://docs.rs/texting_robots/latest/texting_robots/struct.Robot.html#method.allowed

### Enum Sitemap

The [Sitemaps formats][sm-format] of an XML Sitemap page.

| Lua variant     | Description               |
|-----------------|---------------------------|
| Sitemap.INDEX   | A `<sitemapindex>` format |
| Sitemap.URL_SET | A `<urlset>` format       |

[sm-format]: https://en.wikipedia.org/wiki/Sitemaps#File_format
