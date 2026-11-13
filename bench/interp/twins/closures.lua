local v = {}
for k = 0, 999 do v[#v + 1] = k end
local function map(t, f) local o = {} for i = 1, #t do o[i] = f(t[i]) end return o end
local function filter(t, f) local o = {} for i = 1, #t do if f(t[i]) then o[#o + 1] = t[i] end end return o end
local total = 0
for r = 0, 299 do
  local s = 0
  for _, x in ipairs(map(filter(map(v, function(x) return x * r end), function(x) return x % 3 == 0 end), function(x) return x // 3 end)) do s = s + x end
  total = total + s % 100000
end
print(total)
