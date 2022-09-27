sws.seedSitemaps = {
   "https://www.urbandictionary.com/sitemap-https.xml.gz"
}

function acceptUrl(url, context)
   if context:sitemap() == sws.Sitemap.URL_SET then
      return string.find(url, "term=")
   else
      return true
   end
end

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
