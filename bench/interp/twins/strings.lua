local parts, len, total = {}, 0, 0
for i = 0, 199999 do
  local p = (i % 10) .. ","
  parts[#parts + 1] = p
  len = len + #p
  if len > 1000 then total = total + len; parts = {}; len = 0 end
end
print(total .. " " .. #table.concat(parts))
