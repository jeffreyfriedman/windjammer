local sqrt = math.sqrt
local function advance(b, dt)
  local n = #b
  for i = 1, n do
    local bi = b[i]
    for j = i + 1, n do
      local bj = b[j]
      local dx, dy, dz = bi.x - bj.x, bi.y - bj.y, bi.z - bj.z
      local d2 = dx * dx + dy * dy + dz * dz
      local mag = dt / (d2 * sqrt(d2))
      local mj, mi = bj.m, bi.m
      bi.vx = bi.vx - dx * mj * mag; bi.vy = bi.vy - dy * mj * mag; bi.vz = bi.vz - dz * mj * mag
      bj.vx = bj.vx + dx * mi * mag; bj.vy = bj.vy + dy * mi * mag; bj.vz = bj.vz + dz * mi * mag
    end
  end
  for i = 1, n do local p = b[i]; p.x = p.x + dt * p.vx; p.y = p.y + dt * p.vy; p.z = p.z + dt * p.vz end
end
local function energy(b)
  local e = 0.0
  for i = 1, #b do
    local p = b[i]
    e = e + 0.5 * p.m * (p.vx * p.vx + p.vy * p.vy + p.vz * p.vz)
    for j = i + 1, #b do
      local q = b[j]
      local dx, dy, dz = p.x - q.x, p.y - q.y, p.z - q.z
      e = e - p.m * q.m / sqrt(dx * dx + dy * dy + dz * dz)
    end
  end
  return e
end
local function B(x, y, z, vx, vy, vz, m) return { x = x, y = y, z = z, vx = vx, vy = vy, vz = vz, m = m } end
local b = { B(0, 0, 0, 0, 0, 0, 39.47), B(4.84, -1.16, -0.10, 0.60, 2.81, -0.02, 0.037),
  B(8.34, 4.12, -0.40, -1.01, 1.82, 0.008, 0.011), B(12.89, -15.11, -0.22, 1.08, 0.86, -0.01, 0.0017),
  B(15.37, -25.91, 0.17, 0.97, 0.59, -0.03, 0.002) }
local function trunc(x) if x >= 0 then return math.tointeger(math.floor(x)) else return math.tointeger(math.ceil(x)) end end
local before = energy(b)
for s = 1, 20000 do advance(b, 0.01) end
local after = energy(b)
print(trunc(before * 1000000.0) .. " " .. trunc(after * 1000000.0))
