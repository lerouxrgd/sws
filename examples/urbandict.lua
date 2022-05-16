
sitemapUrl = "https://www.urbandictionary.com/sitemap-https.xml.gz"

function acceptUrl(sitemap, url)
   if sitemap:kind() == "Urlset" then
      return string.find(url, "term=")
   else
      return true
   end
end

function processPage(page, context)
   for i, def in sws.selectIter(page:select("section .definition")) do
      local record = sws.newRecord()

      local _, word = sws.selectIter(def:select("h1 a.word"))()
      word = word:innerHtml()
      if string.find(word, "\t") then goto continue end

      local _, contributor = sws.selectIter(def:select(".contributor"))()
      local date = string.match(contributor:innerHtml(), ".*\\?</a>%s*(.*)\\?")

      local _, meaning = sws.selectIter(def:select(".meaning"))()
      meaning = meaning:innerText():gsub("[\n\r]+", " "):gsub("\t+", "")

      local _, example = sws.selectIter(def:select(".example"))()
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
