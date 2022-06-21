sws.seedSitemaps = {
   "https://www.urbandictionary.com/sitemap-https.xml.gz"
}

function acceptUrl(sitemap, url)
   if sitemap == sws.Sitemap.URL_SET then
      return string.find(url, "term=")
   else
      return true
   end
end

function scrapPage(page, context)
   for i, def in page:select("section .definition"):enumerate() do
      local record = sws.Record()

      local word = def:select("h1 a.word"):iter()():innerHtml()
      if string.find(word, "\t") then goto continue end

      local contributor = def:select(".contributor"):iter()()
      local date = string.match(contributor:innerHtml(), ".*\\?</a>%s*(.*)\\?")
      date = sws.Date(date, "%B %d, %Y"):format("%Y-%m-%d")

      local meaning = def:select(".meaning"):iter()()
      meaning = meaning:innerText():gsub("[\n\r]+", " "):gsub("\t+", "")

      local example = def:select(".example"):iter()()
      example = example:innerText():gsub("[\n\r]+", " "):gsub("\t+", "")

      if word and date and meaning and example then
         record:pushField(word)
         record:pushField(tostring(i))
         record:pushField(date)
         record:pushField(meaning)
         record:pushField(example)

         context:sendRecord(record)
      end

      ::continue::
   end
end
