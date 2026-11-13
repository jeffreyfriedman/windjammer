local m, size = {}, 0
for i = 0, 199999 do
  local k = (i * 7919) % 10007
  local c = m[k]
  if c == nil then size = size + 1; c = 0 end
  m[k] = c + 1
end
local best = 0
for j = 0, 10006 do local x = m[j]; if x ~= nil and x > best then best = x end end
print(size .. " " .. best)
