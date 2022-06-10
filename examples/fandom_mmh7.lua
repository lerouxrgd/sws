sws.seedPages = {
   "https://mightandmagic.fandom.com/wiki/Academy_(H7)",
   "https://mightandmagic.fandom.com/wiki/Dungeon_(H7)",
   "https://mightandmagic.fandom.com/wiki/Fortress_(H7)",
   "https://mightandmagic.fandom.com/wiki/Haven_(H7)",
   "https://mightandmagic.fandom.com/wiki/Necropolis_(H7)",
   "https://mightandmagic.fandom.com/wiki/Stronghold_(H7)",
   "https://mightandmagic.fandom.com/wiki/Sylvan_(H7)",
}

function scrapPage(page, ctx)
   local categories = page:select("nav#articleCategories"):iter()()
   for cat in categories:select("li span a"):iter() do
      local cat = cat:innerText()
      if cat == "Heroes VII factions" then
         scrapFaction(page, ctx)
      elseif string.match(cat, "Heroes VII (.+)\\? creatures") then
         scrapCreature(page, ctx)
      end
   end
end

function scrapFaction(page, ctx)
   for creature in page:select("div.tabber table td a:last-of-type"):iter() do
      local url = "https://mightandmagic.fandom.com" .. creature:attr("href")
      ctx:sendUrl(url)
   end
end

function scrapCreature(page, ctx)
   local creature = (page:select("aside h2.pi-item.pi-title"):iter()()):innerText()

   local row = {}
   for data in page:select("aside section.pi-group div.pi-data"):iter() do
      local label = (data:select("h3.pi-data-label"):iter()()):innerText()
      local value = (data:select("div.pi-data-value"):iter()()):innerText()
      if label == "Upgraded" then
         local upgraded = data:select("div.pi-data-value img[alt=Yes]"):iter()()
         value = tostring(upgraded ~= nil)
      end
      row[label] = value:gsub("^%s*(.-)%s*$", "%1")
   end
   for section in page:select("aside section.pi-group section.pi-item"):iter() do
      local labels = {}
      local values = {}
      for i, label in section:select("section.pi-smart-group-head h3"):enumerate() do
         labels[i] = label:innerText()
      end
      for i, value in section:select("section.pi-smart-group-body div.pi-smart-data-value"):enumerate() do
         values[i] = value:innerText():gsub("^%s*(.-)%s*$", "%1")
      end
      for i = 1,#labels do
         row[labels[i]] = values[i]
      end
   end

   local rec = sws.Record:new()
   rec:pushField(row["Faction"])
   rec:pushField(creature)
   rec:pushField(row["Tier/level"])
   rec:pushField(row["Upgraded"] or "N/A")
   rec:pushField(row["Size"] or "N/A")
   rec:pushField(row["Attack type"] or "N/A")
   rec:pushField(row["Range"] or "N/A")
   rec:pushField(row["Dwelling"] or "N/A")
   rec:pushField(row["Cost per unit"] or "N/A")
   rec:pushField(row["Growth"] or "N/A")
   rec:pushField(row["Attack"])
   rec:pushField(row["Defense"])
   rec:pushField(row["Hit Points"])
   rec:pushField(row["Damage"])
   rec:pushField(row["Initiative"])
   rec:pushField(row["Speed"] or "N/A")
   rec:pushField(row["Morale"] or "N/A")
   rec:pushField(row["Destiny"] or "N/A")
   ctx:sendRecord(rec)
end
