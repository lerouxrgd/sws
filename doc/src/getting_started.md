# Getting Started

## Get the binary

Download the latest standalone binary for your OS on the [release][] page, and put it in
a location available in your `PATH`.

[release]: https://github.com/lerouxrgd/sws/releases

## Basic example

Let's create a simple `urbandict.lua` scraper for [Urban Dictionary][ud]. Copy paste the
following command:

```sh
cat << 'EOF' > urbandict_demo.lua
sws.seedPages = {
   "https://www.urbandictionary.com/define.php?term=Lua"
}

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
EOF
```

You can then run it with:

```sh
sws crawl --script urbandict_demo.lua
```

As we have defined `sws.seedPages` to be a single page (that is [Urban Dictionary's
Lua][ud-lua] definition), the `scrapPage` function will be run on that single page
only. There are multiple seeding options which are detailed in the [Lua scraper - Seed
definition][lua-scraper] section.

By default the resulting csv file is written to stdout, however the `-o` (or
`--output-file`) lets us specify a proper output file. Note that this file can be also
be appended or truncated, using the additional flags `--append` or `--truncate`
respectively. See the [crawl subcommand][crawl-doc] section for me details.

[ud]: https://www.urbandictionary.com/
[ud-lua]: https://www.urbandictionary.com/define.php?term=Lua
[lua-scraper]: ./lua_scraper.html#seed-definition
[crawl-doc]: ./crawl_overview.html

## Bash completion

You can source the completion script in your `~/.bashrc` file with:

```bash
echo 'source <(sws completion)' >> ~/.bashrc
```
