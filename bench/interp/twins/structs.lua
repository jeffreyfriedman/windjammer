local V2 = {}
V2.__index = V2
local function new(x, y) return setmetatable({ x = x, y = y }, V2) end
function V2:add(o) return new(self.x + o.x, self.y + o.y) end
function V2:dot(o) return self.x * o.x + self.y * o.y end
local acc = new(0, 0)
local s = 0
for i = 0, 299999 do
  local v = new(i % 13, i % 7)
  acc = acc:add(v)
  s = s + acc:dot(v) % 1000
end
print(acc.x .. " " .. acc.y .. " " .. s)
