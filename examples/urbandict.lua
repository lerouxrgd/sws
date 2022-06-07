
sws.sitemapUrl = "https://www.urbandictionary.com/sitemap-https.xml.gz"

function acceptUrl(sitemap, url)
   if sitemap == sws.Sitemap.URL_SET then
      return string.find(url, "term=")
   else
      return true
   end
end

function scrapPage(page, context)
   for i, def in sws.enumerate(page:select("section .definition")) do
      local record = sws.Record:new()

      local word = sws.iter(def:select("h1 a.word"))()
      word = word:innerHtml()
      if string.find(word, "\t") then goto continue end

      local contributor = sws.iter(def:select(".contributor"))()
      local date = string.match(contributor:innerHtml(), ".*\\?</a>%s*(.*)\\?")

      local meaning = sws.iter(def:select(".meaning"))()
      meaning = meaning:innerText():gsub("[\n\r]+", " "):gsub("\t+", "")

      local example = sws.iter(def:select(".example"))()
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
