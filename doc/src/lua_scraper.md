# Lua Scraper

The scraping logic is configured through a single `Lua` script.

The customizable parameters are:
* [seed](#seed-definition): Defines the seed pages for crawling
* [acceptUrl](#function-accepturl): A function to specify whether to accept a URL
  when crawling an [XML Sitemap][xml-sitemap]
* [scrapPage](#function-scrappage): A function that defines the scraping logic for a
  single HTML page

[xml-sitemap]: https://en.wikipedia.org/wiki/Site_map

## Seed definition

The [seed](./lua_api_overview.html#seeds) be one of `seedSitemaps`, `seedPages`, or
`seedRobotsTxt`.

Defining a `seed` is always **mandatory**. However, when using the [scrap
subcommand](./scrap_overview.html) it will be ignored as the input will be either the
specified URL or the specified local files.

⚠️ Defining multiple `seeds` will throw an **error** ⚠️

### Example

```lua
-- A list of sitemap URLs (gzipped sitemaps are supported)
sws.seedSitemaps = {
   "https://www.urbandictionary.com/sitemap-https.xml.gz"
}
```

```lua
-- A list of HTML pages
sws.seedPages = {
   "https://www.urbandictionary.com/define.php?term=Rust",
   "https://www.urbandictionary.com/define.php?term=Lua",
}
```

```lua
-- A single robots.txt URL
sws.seedRobotsTxt = "https://www.urbandictionary.com/robots.txt"
```

## Robot definition

A [robots.txt][robots-txt] can be used either as:

* A crawling seed through `sws.seedRobotsTxt` (see [above](#example))

* A URL validation helper through `sws.crawlerConfig`'s parameter `robot` (see [crawler
  configuration](./crawl_config.html))

In both cases, the resulting [Robot](./lua_api_overview.html#class-robot) can be used to
check whether a given URL is crawlable. This `Robot` is available through both
[CrawlingContext](./lua_api_overview.html#class-crawlingcontext) (in
[acceptUrl](#function-accepturl)), and
[ScrapingContext](./lua_api_overview.html#class-scrapingcontext) (in
[scrapPage](#function-scrappage)).

The underlying `Robot` implementation in Rust is using the crate
[texting_robots][robots-rs].

Defining a `robot` is **optional**.

[robots-txt]: https://en.wikipedia.org/wiki/Robots.txt
[robots-rs]: https://docs.rs/texting_robots/latest/texting_robots/index.html

## Function acceptUrl

```lua
function acceptUrl(url, context)
```

A `Lua` function to specify whether to accept a URL when crawling an [XML
Sitemap][xml-sitemap]. Its parameters are:

* **url:** A URL `string` that is a candidate for crawling/scraping

* **context:** An instance of
  [CrawlingContext](./lua_api_overview.html#class-crawlingcontext)

Defining `acceptUrl` is **optional**.

[sm-format]: https://en.wikipedia.org/wiki/Sitemaps#File_format

### Example

From `examples/urbandict.lua`:

```lua
function acceptUrl(url, context)
   if context:sitemap() == sws.Sitemap.URL_SET then
      return string.find(url, "term=")
   else
      -- For sws.Sitemap.INDEX accept all entries
      return true
   end
end
```

## Function scrapPage

```lua
function scrapPage(page, context)
```

A `Lua` function that defines the scraping logic for a single page. Its parameters are:

* **page:** The [Html](./lua_api_overview.html#class-html) page being scraped

* **context:** An instance of
  [ScrapingContext](./lua_api_overview.html#class-scrapingcontext)

Defining `scrapPage` is **mandatory**.

#### CSS Selectors

CSS selectors are the most powerful feature of this scraper, they are used to target and
extract HTML elements in a flexible and efficient way. You can read more about CSS
selectors on [MDN doc][css-sel-mdn], and find a good reference on [W3C
doc][css-sel-w3c].

```lua
function scrapPage(page, context)
   for i, def in page:select("section .definition"):enumerate() do
      local word = def:select("h1 a.word"):iter()()
      print(string.format("Definition %i: %s", i, word))
   end
end
```

The `select` method is expecting a CSS selector string, its result can be either
iterated or enumerated with `iter` and `enumerate` respectively. Interestingly, the
elements being iterated over allow for sub selection as they also have a `select`
method, this enables very flexible HTML elements selection.

See more details in the reference for the [Select](./lua_api_overview.html#class-select)
class.

[css-sel-mdn]: https://developer.mozilla.org/en-US/docs/Learn/CSS/Building_blocks/Selectors
[css-sel-w3c]: https://www.w3schools.com/cssref/css_selectors.php

#### Utils

Some utility functions are also exposed in `Lua`.

* Date utils:

  The [Date](./lua_api_overview.html#class-date) helper can parse and format dates:

  ```lua
  local date = "March 18, 2005" -- Extracted from some page's element
  date = sws.Date(date, "%B %d, %Y"):format("%Y-%m-%d") -- Now date is "2005-03-18"
  ```

  Under the hood a `Date` wraps a Rust [chrono::NaiveDate][chrono-date] that is created
  using [NaiveDate::parse_from_str][chrono-fmt]. The `format` method will return a
  string formatted with the specified format (see [specifiers][chrono-specifiers] for
  the formatting options).

[chrono-date]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html
[chrono-fmt]: https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.parse_from_str
[chrono-specifiers]: https://docs.rs/chrono/latest/chrono/format/strftime/index.html

### Example

From `examples/urbandict.lua`:

```lua
function scrapPage(page, context)
   for defIndex, def in page:select("section .definition"):enumerate() do
      local word = def:select("h1 a.word"):iter()()
      if not word then
         word = def:select("h2 a.word"):iter()()
      end
      if not word then
         goto continue
      end
      word = word:innerHtml()

      local contributor = def:select(".contributor"):iter()()
      local date = string.match(contributor:innerHtml(), ".*\\?</a>%s*(.*)\\?")
      date = sws.Date(date, "%B %d, %Y"):format("%Y-%m-%d")

      local meaning = def:select(".meaning"):iter()()
      meaning = meaning:innerText():gsub("[\n\r]+", " ")

      local example = def:select(".example"):iter()()
      example = example:innerText():gsub("[\n\r]+", " ")

      if word and date and meaning and example then
         local record = sws.Record()
         record:pushField(word)
         record:pushField(defIndex)
         record:pushField(date)
         record:pushField(meaning)
         record:pushField(example)
         context:sendRecord(record)
      end

      ::continue::
   end
end
```

## CSV Record

The Lua [Record](./lua_api_overview.html#class-record) class wraps a Rust
[`csv::StringRecord`][csv-string-rec] struct. In `Lua` it can be instantiated through
`sws.Record()`. Its `pushField(someString)` method should be used to add string fields
to the record.

It is possible to customize the underlying [CSV Writer][csv-writer] in `Lua` through the
`sws.csvWriterConfig` table.

| csv::WriterBuilder method    | Lua parameter | Example Lua value | Default Lua value |
|------------------------------|---------------|-------------------|-------------------|
| [delimiter][csv-delimiter]   | delimiter     | "\t"              | ","               |
| [escape][csv-escape]         | escape        | ";"               | "\\""             |
| [flexible][csv-flexible]     | flexible      | true              | false             |
| [terminator][csv-terminator] | terminator    | CRLF              | { Any = "\n" }    |

[csv-string-rec]: https://docs.rs/csv/latest/csv/struct.StringRecord.html
[csv-writer]: https://docs.rs/csv/latest/csv/struct.Writer.html
[csv-delimiter]: https://docs.rs/csv/latest/csv/struct.WriterBuilder.html#method.delimiter
[csv-escape]: https://docs.rs/csv/latest/csv/struct.WriterBuilder.html#method.escape
[csv-flexible]: https://docs.rs/csv/latest/csv/struct.WriterBuilder.html#method.flexible
[csv-terminator]: https://docs.rs/csv/latest/csv/struct.WriterBuilder.html#method.terminator

### Example

```lua
sws.csvWriterConfig = {
   delimiter = "\t"
}

function scrapPage(page, context)
    local record = sws.Record()
    record:pushField("foo field")
    record:pushField("bar field")
    context:sendRecord(record)
end
```
