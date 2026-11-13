local total = 0
for i = 0, 4999999 do
  local x = i * 3 + 1
  if x % 7 == 0 then total = total + x else total = total - 1 end
end
print(total)
