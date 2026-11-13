local n = 1000000
local flags = {}
for i = 0, n - 1 do flags[i] = true end
local count = 0
for i = 2, n - 1 do
  if flags[i] then
    count = count + 1
    for j = i * i, n - 1, i do flags[j] = false end
  end
end
print(count)
